<?php
/**
 * WordPress Archive Viewer — Serialization-Safe Search & Replace
 *
 * Performs find-and-replace across all tables in a WordPress database,
 * properly handling PHP serialized strings to prevent data corruption.
 *
 * Usage:
 *   php search-replace.php <host> <port> <user> <password> <database> <search> <replace>
 *
 * Based on the interconnect/it search-replace-db approach.
 */

if (php_sapi_name() !== 'cli') {
    die('This script must be run from the command line.');
}

if ($argc < 8) {
    echo "Usage: php search-replace.php <host> <port> <user> <password> <database> <search> <replace>\n";
    exit(1);
}

$db_host    = $argv[1];
$db_port    = (int)$argv[2];
$db_user    = $argv[3];
$db_pass    = $argv[4];
$db_name    = $argv[5];
$search     = $argv[6];
$replace    = $argv[7];

echo "=== WordPress Search & Replace ===\n";
echo "Database: $db_name @ $db_host:$db_port\n";
echo "Search:   $search\n";
echo "Replace:  $replace\n\n";

// Connect to the database
$mysqli = new mysqli($db_host, $db_user, $db_pass, $db_name, $db_port);
if ($mysqli->connect_error) {
    echo "ERROR: Connection failed: " . $mysqli->connect_error . "\n";
    exit(1);
}
$mysqli->set_charset('utf8mb4');

// Get all tables
$tables = [];
$result = $mysqli->query("SHOW TABLES");
while ($row = $result->fetch_row()) {
    $tables[] = $row[0];
}
$result->free();

echo "Found " . count($tables) . " tables.\n\n";

$total_changes = 0;

foreach ($tables as $table) {
    // Get columns
    $columns = [];
    $primary_keys = [];

    $result = $mysqli->query("DESCRIBE `$table`");
    while ($row = $result->fetch_assoc()) {
        $columns[] = $row['Field'];
        if ($row['Key'] === 'PRI') {
            $primary_keys[] = $row['Field'];
        }
    }
    $result->free();

    if (empty($primary_keys)) {
        // Skip tables without primary keys
        continue;
    }

    // Build select query
    $pk_cols = implode('`, `', $primary_keys);
    $all_cols = implode('`, `', $columns);

    // Find rows containing the search string in any text column
    $text_columns = [];
    $result = $mysqli->query("SHOW COLUMNS FROM `$table`");
    while ($row = $result->fetch_assoc()) {
        $type = strtolower($row['Type']);
        if (strpos($type, 'text') !== false ||
            strpos($type, 'varchar') !== false ||
            strpos($type, 'char') !== false ||
            strpos($type, 'blob') !== false ||
            strpos($type, 'longtext') !== false ||
            strpos($type, 'mediumtext') !== false) {
            $text_columns[] = $row['Field'];
        }
    }
    $result->free();

    if (empty($text_columns)) {
        continue;
    }

    // Build WHERE clause to find rows with the search string
    $where_parts = [];
    foreach ($text_columns as $col) {
        $escaped_search = $mysqli->real_escape_string($search);
        $where_parts[] = "`$col` LIKE '%$escaped_search%'";
    }
    $where = implode(' OR ', $where_parts);

    $sql = "SELECT `$pk_cols`, `$all_cols` FROM `$table` WHERE $where";
    $result = $mysqli->query($sql);

    if (!$result || $result->num_rows === 0) {
        if ($result) $result->free();
        continue;
    }

    $table_changes = 0;

    while ($row = $result->fetch_assoc()) {
        $updates = [];
        $row_changed = false;

        foreach ($text_columns as $col) {
            if (!isset($row[$col]) || $row[$col] === null) {
                continue;
            }

            $old_value = $row[$col];
            $new_value = recursive_unserialize_replace($search, $replace, $old_value);

            if ($old_value !== $new_value) {
                $updates[$col] = $new_value;
                $row_changed = true;
            }
        }

        if ($row_changed) {
            // Build UPDATE query
            $set_parts = [];
            foreach ($updates as $col => $val) {
                $escaped_val = $mysqli->real_escape_string($val);
                $set_parts[] = "`$col` = '$escaped_val'";
            }

            $pk_where = [];
            foreach ($primary_keys as $pk) {
                $escaped_pk = $mysqli->real_escape_string($row[$pk]);
                $pk_where[] = "`$pk` = '$escaped_pk'";
            }

            $update_sql = "UPDATE `$table` SET " . implode(', ', $set_parts) .
                         " WHERE " . implode(' AND ', $pk_where) . " LIMIT 1";

            if ($mysqli->query($update_sql)) {
                $table_changes++;
            } else {
                echo "  WARNING: Failed to update row in $table: " . $mysqli->error . "\n";
            }
        }
    }
    $result->free();

    if ($table_changes > 0) {
        echo "  [$table] $table_changes row(s) updated\n";
        $total_changes += $table_changes;
    }
}

echo "\n=== Complete: $total_changes total row(s) updated ===\n";
$mysqli->close();
exit(0);


/**
 * Recursively handle serialized data, performing the search-replace
 * while maintaining correct serialization string lengths.
 */
function recursive_unserialize_replace($search, $replace, $data, $serialised = false) {
    // Attempt to unserialize the data
    $unserialized = @unserialize($data);

    if ($unserialized !== false || $data === 'b:0;') {
        // Data was serialized — recurse into it
        if (is_array($unserialized)) {
            $new_data = [];
            foreach ($unserialized as $key => $value) {
                $new_data[$key] = recursive_unserialize_replace($search, $replace, $value, false);
            }
            return serialize($new_data);
        } elseif (is_object($unserialized)) {
            $props = get_object_vars($unserialized);
            foreach ($props as $key => $value) {
                $unserialized->$key = recursive_unserialize_replace($search, $replace, $value, false);
            }
            return serialize($unserialized);
        } elseif (is_string($unserialized)) {
            $replaced = str_replace($search, $replace, $unserialized);
            return serialize($replaced);
        }
        return serialize($unserialized);
    }

    // Data is not serialized — do simple replacement
    if (is_string($data)) {
        return str_replace($search, $replace, $data);
    }

    return $data;
}

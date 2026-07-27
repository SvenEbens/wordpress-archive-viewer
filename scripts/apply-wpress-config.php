<?php
/**
 * WordPress Archive Viewer - Apply WPRESS Config
 *
 * Reads package.json from a .wpress archive and applies the active plugins
 * and theme to the database.
 *
 * Usage: php apply-wpress-config.php <package_json_path> <host> <port> <user> <pass> <db> <prefix>
 */

if (php_sapi_name() !== 'cli') {
    die('This script must be run from the command line.');
}

if ($argc < 8) {
    echo "Usage: php apply-wpress-config.php <package_json_path> <host> <port> <user> <pass> <db> <prefix>\n";
    exit(1);
}

$package_path = $argv[1];
$db_host = $argv[2];
$db_port = (int)$argv[3];
$db_user = $argv[4];
$db_pass = $argv[5];
$db_name = $argv[6];
$prefix = $argv[7];

if (!file_exists($package_path)) {
    echo "No package.json found. Skipping.\n";
    exit(0);
}

$json_str = file_get_contents($package_path);
$data = json_decode($json_str, true);

if (!$data) {
    echo "Failed to parse package.json.\n";
    exit(1);
}

$mysqli = new mysqli($db_host, $db_user, $db_pass, $db_name, $db_port);
if ($mysqli->connect_error) {
    echo "ERROR: Connection failed: " . $mysqli->connect_error . "\n";
    exit(1);
}
$mysqli->set_charset('utf8mb4');

// Helper function to update or insert option
function update_option($mysqli, $prefix, $option_name, $option_value) {
    $stmt = $mysqli->prepare("SELECT option_id FROM {$prefix}options WHERE option_name = ?");
    if (!$stmt) return;
    $stmt->bind_param("s", $option_name);
    $stmt->execute();
    $result = $stmt->get_result();
    $exists = $result->num_rows > 0;
    $stmt->close();

    if ($exists) {
        $stmt = $mysqli->prepare("UPDATE {$prefix}options SET option_value = ? WHERE option_name = ?");
        if ($stmt) {
            $stmt->bind_param("ss", $option_value, $option_name);
            $stmt->execute();
            $stmt->close();
        }
    } else {
        $stmt = $mysqli->prepare("INSERT INTO {$prefix}options (option_name, option_value, autoload) VALUES (?, ?, 'yes')");
        if ($stmt) {
            $stmt->bind_param("ss", $option_name, $option_value);
            $stmt->execute();
            $stmt->close();
        }
    }
}

if (!empty($data['Plugins']) && is_array($data['Plugins'])) {
    // AIO WP Migration package.json stores plugins as an array of plugin paths
    // Let's ensure it's a numeric array for active_plugins
    $plugins = array_values($data['Plugins']);
    $serialized = serialize($plugins);
    update_option($mysqli, $prefix, 'active_plugins', $serialized);
    echo "Activated " . count($plugins) . " plugins.\n";
}

if (!empty($data['Template'])) {
    update_option($mysqli, $prefix, 'template', $data['Template']);
    echo "Set template to " . $data['Template'] . "\n";
}

if (!empty($data['Stylesheet'])) {
    update_option($mysqli, $prefix, 'stylesheet', $data['Stylesheet']);
    echo "Set stylesheet to " . $data['Stylesheet'] . "\n";
}

$mysqli->close();
echo "WPRESS config applied.\n";
exit(0);

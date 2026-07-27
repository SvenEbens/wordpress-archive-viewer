import { AppState, stopServer, showToast } from './app.js';

let invoke, open;

function initTauriIfNeeded() {
    if (!invoke && window.__TAURI__) {
        invoke = window.__TAURI__.core?.invoke;
        
        // Try to get open from Tauri plugin, otherwise fallback to standard window.open
        open = window.__TAURI_PLUGIN_SHELL__?.open 
            || window.__TAURI__?.plugin?.shell?.open 
            || window.__TAURI__?.shell?.open
            || ((url) => window.open(url, '_blank'));
    } else if (!invoke) {
        open = (url) => window.open(url, '_blank');
    }
}

export function initStatus(siteInfo) {
    initTauriIfNeeded();
    if (!siteInfo) return;

    // Update Header
    document.getElementById('running-title').textContent = siteInfo.name;
    document.getElementById('running-url').textContent = siteInfo.url;

    // Update Details
    document.getElementById('detail-wp-version').textContent = siteInfo.wordpress_version || 'Unknown';
    document.getElementById('detail-php-version').textContent = siteInfo.php_version || 'Unknown';
    document.getElementById('detail-port').textContent = siteInfo.port || 'Unknown';
    document.getElementById('detail-created').textContent = siteInfo.created || 'Unknown';

    // Ensure all dots are running state
    ['db', 'php', 'server'].forEach(svc => {
        const statusEl = document.getElementById(`svc-${svc}-status`);
        const dotEl = document.getElementById(`svc-${svc}-dot`);
        if (statusEl) statusEl.textContent = 'Running';
        if (dotEl) {
            dotEl.className = 'service-dot running';
        }
    });

    // Set up button listeners
    const btnBrowser = document.getElementById('btn-browser');
    const btnAdmin = document.getElementById('btn-admin');
    const btnStop = document.getElementById('btn-stop');

    // Remove existing listeners (cloning is a quick way)
    const newBtnBrowser = btnBrowser.cloneNode(true);
    btnBrowser.parentNode.replaceChild(newBtnBrowser, btnBrowser);
    
    const newBtnAdmin = btnAdmin.cloneNode(true);
    btnAdmin.parentNode.replaceChild(newBtnAdmin, btnAdmin);
    
    const newBtnStop = btnStop.cloneNode(true);
    btnStop.parentNode.replaceChild(newBtnStop, btnStop);

    newBtnBrowser.addEventListener('click', async () => {
        try {
            await open(siteInfo.url);
        } catch (e) {
            showToast('Failed to open browser', 'error');
        }
    });

    newBtnAdmin.addEventListener('click', async () => {
        try {
            await open(`${siteInfo.url}/wp-admin/`);
        } catch (e) {
            showToast('Failed to open browser', 'error');
        }
    });

    newBtnStop.addEventListener('click', () => {
        newBtnStop.disabled = true;
        newBtnStop.textContent = 'Stopping...';
        stopServer().finally(() => {
            newBtnStop.disabled = false;
            newBtnStop.textContent = 'Stop Server';
        });
    });
}

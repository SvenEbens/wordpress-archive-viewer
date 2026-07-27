import { initSplash } from './splash.js';
import { initArchives } from './archives.js';
import { initStatus } from './status.js';

let invoke, listen, open;

function initTauri() {
    if (window.__TAURI__) {
        invoke = window.__TAURI__.core.invoke;
        listen = window.__TAURI__.event.listen;
        // shell open is used in status.js, but since status.js imports from app, let's pass or handle it there.
    } else {
        throw new Error("Tauri API not found! Ensure 'withGlobalTauri': true is in tauri.conf.json");
    }
}

export const AppState = {
    currentView: 'splash', // splash, selector, running, error
    archives: [],
    selectedArchive: null,
    services: {
        db: 'stopped', // stopped, starting, running, error
        php: 'stopped',
        server: 'stopped'
    },
    siteInfo: null
};

export function switchView(viewId) {
    document.querySelectorAll('.view').forEach(el => el.classList.remove('active'));
    document.getElementById(`view-${viewId}`).classList.add('active');
    AppState.currentView = viewId;
}

export function showToast(message, type = 'info') {
    const container = document.getElementById('toast-container');
    const toast = document.createElement('div');
    toast.className = `toast toast-${type}`;
    toast.textContent = message;

    container.appendChild(toast);

    setTimeout(() => {
        toast.classList.add('toast-out');
        setTimeout(() => toast.remove(), 300);
    }, 4000);
}

async function fetchArchives() {
    try {
        document.getElementById('splash-message').textContent = 'Loading archives...';
        console.log("Calling get_archives on backend...");
        const archives = await invoke('get_archives');
        console.log("Got archives:", archives);
        
        AppState.archives = archives;
        initArchives(archives);
        
        if (archives.length === 0) {
            // Show empty state in selector
            switchView('selector');
        } else if (archives.length === 1) {
            // Auto-start if only one archive
            AppState.selectedArchive = archives[0].id;
            startArchive(archives[0].id);
        } else {
            // Show selector
            switchView('selector');
        }
    } catch (error) {
        console.error("fetchArchives error:", error);
        document.getElementById('splash-message').textContent = 'Error: ' + error;
        showError('Failed to load archives: ' + error);
    }
}

export async function startArchive(archiveId, forceReextract = false) {
    try {
        switchView('splash');
        
        // Let the splash screen know we are starting a specific archive
        document.getElementById('splash-message').textContent = 'Starting archive...';
        
        await invoke('start_archive', { id: archiveId, forceReextract: forceReextract });
        // The backend will emit status events, transitioning the UI to 'running'
        // when all services are ready.
    } catch (error) {
        showError('Failed to start archive: ' + error);
    }
}

export async function stopServer() {
    try {
        await invoke('stop_all');
        switchView('selector');
    } catch (error) {
        showError('Failed to stop services: ' + error);
    }
}

export function showError(message) {
    document.getElementById('error-message').textContent = message;
    switchView('error');
    console.error(message);
}

async function setupEventListeners() {
    document.getElementById('btn-back').addEventListener('click', () => {
        switchView('selector');
    });

    document.getElementById('btn-retry').addEventListener('click', () => {
        if (AppState.selectedArchive) {
            const forceReextract = document.getElementById('chk-force-reextract')?.checked || false;
            startArchive(AppState.selectedArchive, forceReextract);
        } else {
            fetchArchives();
        }
    });

    // Listen for backend status updates
    await listen('status_update', (event) => {
        const payload = event.payload;
        // payload format: { service: 'db', status: 'running', message: '...' }
        // or { status: 'all_ready', site_info: {...} }
        
        if (payload.status === 'all_ready') {
            AppState.siteInfo = payload.site_info;
            initStatus(AppState.siteInfo);
            switchView('running');
        } else if (payload.service) {
            AppState.services[payload.service] = payload.status;
            
            // Update splash screen if it's active
            if (AppState.currentView === 'splash') {
                const el = document.getElementById(`status-${payload.service}`);
                if (el) {
                    el.className = `status-item ${payload.status}`;
                }
            }
        }
    });
}

async function init() {
    console.log("App init starting...");
    initSplash();
    
    try {
        initTauri();
    } catch (e) {
        document.getElementById('splash-message').textContent = e.message;
        document.getElementById('splash-message').style.color = '#EF4444';
        console.error(e);
        return;
    }
    
    await setupEventListeners();
    
    // Simulate loading time for visual effect (backend will likely be fast)
    setTimeout(() => {
        console.log("Timeout finished, fetching archives...");
        fetchArchives();
    }, 1000);
}

document.addEventListener('DOMContentLoaded', init);

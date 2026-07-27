import { AppState, startArchive } from './app.js';

export function initArchives(archives) {
    const grid = document.getElementById('archive-grid');
    const openBtn = document.getElementById('btn-open');
    
    grid.innerHTML = '';
    
    if (archives.length === 0) {
        grid.innerHTML = '<div style="grid-column: 1/-1; text-align: center; padding: 40px; color: var(--text-secondary);">No archives found. Add a .zip file to the archives folder.</div>';
        return;
    }

    archives.forEach(archive => {
        const card = document.createElement('div');
        card.className = 'archive-card';
        card.dataset.id = archive.id;
        
        // Use placeholder if no thumbnail
        const thumbUrl = archive.thumbnail ? `asset://localhost/archives/${archive.id}/${archive.thumbnail}` : 'assets/placeholder.png';
        
        card.innerHTML = `
            <div class="archive-thumb">
                <img src="${thumbUrl}" alt="Thumbnail" onerror="this.src='data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxMDAlIiBoZWlnaHQ9IjEwMCUiPjxyZWN0IHdpZHRoPSIxMDAlIiBoZWlnaHQ9IjEwMCUiIGZpbGw9IiMzMzMiLz48dGV4dCB4PSI1MCUiIHk9IjUwJSIgZm9udC1mYW1pbHk9InNhbnMtc2VyaWYiIGZvbnQtc2l6ZT0iMTQiIGZpbGw9IiM2NjYiIHRleHQtYW5jaG9yPSJtaWRkbGUiIGR5PSIuM2VtIj5ObyBJbWFnZTwvdGV4dD48L3N2Zz4='">
            </div>
            <div class="archive-content">
                <div class="archive-title" title="${archive.name}">${archive.name}</div>
                <div class="archive-desc" title="${archive.description || ''}">${archive.description || 'No description provided'}</div>
                <div class="archive-meta">
                    <span>${archive.created || 'Unknown date'}</span>
                    <span class="badge wp">WP ${archive.wordpress_version || '?'}</span>
                </div>
            </div>
        `;

        card.addEventListener('click', () => {
            // Deselect others
            document.querySelectorAll('.archive-card').forEach(c => c.classList.remove('selected'));
            // Select this one
            card.classList.add('selected');
            AppState.selectedArchive = archive.id;
            openBtn.disabled = false;
        });

        // Double click to open immediately
        card.addEventListener('dblclick', () => {
            const forceReextract = document.getElementById('chk-force-reextract')?.checked || false;
            startArchive(archive.id, forceReextract);
        });

        grid.appendChild(card);
    });

    openBtn.addEventListener('click', () => {
        if (AppState.selectedArchive) {
            const forceReextract = document.getElementById('chk-force-reextract')?.checked || false;
            startArchive(AppState.selectedArchive, forceReextract);
        }
    });
}

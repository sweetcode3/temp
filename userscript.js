function toggleControls(show) {
    // Find original elements
    const originalFileName = metadataContainer.querySelector('[class*="_file-name"]');
    const githubBtn = controlsDiv.querySelector('[data-tooltip="Save to GitHub"]');
    
    // Toggle GitHub controls
    [repoSelect, filenameInput, saveBtn, cancelBtn].forEach(el => 
        el.style.display = show ? 'block' : 'none'
    );
    
    // Toggle original elements
    originalFileName.style.display = show ? 'none' : 'block';
    githubBtn.style.display = show ? 'none' : 'block';
}

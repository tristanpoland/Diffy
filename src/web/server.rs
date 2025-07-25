use crate::core::{DiffyCore, types::{DiffResult, FileDiff}};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, get_service},
    Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::services::ServeDir;
use anyhow::Result;

#[derive(Clone)]
pub struct AppState {
    pub core: Arc<DiffyCore>,
}

#[derive(Deserialize)]
pub struct FileQuery {
    path: String,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

pub fn create_app(core: DiffyCore) -> Router {
    let state = AppState {
        core: Arc::new(core),
    };

    Router::new()
        .route("/", get(index_handler))
        .route("/api/diff", get(diff_handler))
        .route("/api/file", get(file_diff_handler))
        .nest_service("/static", get_service(ServeDir::new("static")))
        .with_state(state)
}

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn diff_handler(State(state): State<AppState>) -> Result<Json<ApiResponse<DiffResult>>, StatusCode> {
    match state.core.analyze() {
        Ok(result) => Ok(Json(ApiResponse::success(result))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

async fn file_diff_handler(
    Query(params): Query<FileQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<FileDiff>>, StatusCode> {
    let path = PathBuf::from(&params.path);
    match state.core.get_file_diff(&path) {
        Ok(diff) => Ok(Json(ApiResponse::success(diff))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Diffy - Directory & File Diff Tool</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background-color: #000000;
            color: #ffffff;
            height: 100vh;
            overflow: hidden;
        }

        .container {
            display: flex;
            height: 100vh;
        }

        .file-tree {
            width: 250px;
            min-width: 200px;
            max-width: 300px;
            background-color: #1a1a1a;
            border-right: 1px solid #333;
            overflow-y: auto;
        }

        .file-tree-header {
            padding: 10px;
            background-color: #252526;
            border-bottom: 1px solid #333;
            font-weight: bold;
        }

        .file-tree-content {
            padding: 5px;
        }

        .file-item {
            padding: 4px 8px;
            cursor: pointer;
            border-radius: 3px;
            font-size: 13px;
            display: flex;
            align-items: center;
            margin: 1px 0;
        }

        .file-item:hover {
            background-color: #2a2d2e;
        }

        .file-item.selected {
            background-color: #094771;
        }

        .file-icon {
            margin-right: 6px;
            width: 16px;
            text-align: center;
        }

        .tree-connector {
            color: #6e7681;
            font-family: monospace;
            margin-right: 4px;
        }

        .status-icon {
            margin-right: 4px;
            font-weight: bold;
            width: 12px;
        }

        .status-added { color: #4caf50; }
        .status-removed { color: #f44336; }
        .status-modified { color: #ff9800; }
        .status-unchanged { color: #9e9e9e; }

        .diff-panel {
            flex: 1;
            display: flex;
            flex-direction: column;
        }

        .diff-header {
            padding: 10px;
            background-color: #252526;
            border-bottom: 1px solid #333;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .diff-controls {
            display: flex;
            align-items: center;
            gap: 20px;
        }

        .mode-toggle {
            display: flex;
            border: 1px solid #333;
            border-radius: 4px;
            overflow: hidden;
        }

        .mode-btn {
            background-color: #1a1a1a;
            color: #ffffff;
            border: none;
            padding: 6px 12px;
            cursor: pointer;
            font-size: 12px;
            transition: background-color 0.2s;
        }

        .mode-btn:hover {
            background-color: #2a2d2e;
        }

        .mode-btn.active {
            background-color: #094771;
            color: #ffffff;
        }

        .mode-btn:not(:last-child) {
            border-right: 1px solid #333;
        }

        .diff-content {
            flex: 1;
            display: flex;
        }


        .monaco-editor-container {
            flex: 1;
            position: relative;
        }

        .loading {
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100%;
            color: #9e9e9e;
        }

        .welcome {
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100%;
            flex-direction: column;
            color: #9e9e9e;
        }

        .stats {
            display: flex;
            gap: 20px;
            font-size: 12px;
        }

        .stat-item {
            display: flex;
            align-items: center;
            gap: 4px;
        }

        .error {
            color: #f44336;
            padding: 20px;
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="file-tree">
            <div class="file-tree-header">
                Files
            </div>
            <div class="file-tree-content" id="fileTree">
                <div class="loading">Loading...</div>
            </div>
        </div>
        
        <div class="diff-panel">
            <div class="diff-header">
                <h2 id="currentFile">Select a file to view diff</h2>
                <div class="diff-controls">
                    <div class="mode-toggle">
                        <button id="sideBySideBtn" class="mode-btn active">Side-by-Side</button>
                        <button id="unifiedBtn" class="mode-btn">Unified</button>
                    </div>
                    <div class="stats" id="stats"></div>
                </div>
            </div>
            <div class="diff-content">
                <div id="diffEditor" style="width: 100%; height: 100%;"></div>
            </div>
        </div>
    </div>

    <script src="https://unpkg.com/monaco-editor@0.45.0/min/vs/loader.js"></script>
    <script>
        let diffEditor;
        let diffResult = null;
        let currentDiff = null;
        let diffMode = 'side-by-side'; // 'side-by-side' or 'unified'

        require.config({ paths: { 'vs': 'https://unpkg.com/monaco-editor@0.45.0/min/vs' }});
        require(['vs/editor/editor.main'], function() {
            monaco.editor.defineTheme('amoled-dark', {
                base: 'vs-dark',
                inherit: true,
                rules: [],
                colors: {
                    'editor.background': '#000000',
                    'editor.foreground': '#ffffff',
                    'editorLineNumber.foreground': '#6e7681',
                    'editor.selectionBackground': '#264f78',
                    'editor.inactiveSelectionBackground': '#3a3d41',
                    'editorIndentGuide.background': '#404040',
                    'editorIndentGuide.activeBackground': '#707070',
                    'editor.selectionHighlightBackground': '#add6ff26',
                    'diffEditor.insertedTextBackground': '#9ccc2c33',
                    'diffEditor.removedTextBackground': '#ff000033',
                    'diffEditor.insertedLineBackground': '#9ccc2c22',
                    'diffEditor.removedLineBackground': '#ff000022'
                }
            });

            const diffEditorOptions = {
                theme: 'amoled-dark',
                readOnly: true,
                automaticLayout: true,
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                fontSize: 13,
                lineNumbers: 'on',
                renderWhitespace: 'selection',
                wordWrap: 'off',
                renderSideBySide: true,
                ignoreTrimWhitespace: false,
                renderIndicators: true
            };

            diffEditor = monaco.editor.createDiffEditor(document.getElementById('diffEditor'), diffEditorOptions);

            // Set initial content
            const originalModel = monaco.editor.createModel('Select a file to view its diff', 'text');
            const modifiedModel = monaco.editor.createModel('Select a file to view its diff', 'text');
            diffEditor.setModel({
                original: originalModel,
                modified: modifiedModel
            });

            // Set up mode toggle handlers
            document.getElementById('sideBySideBtn').addEventListener('click', () => {
                setDiffMode('side-by-side');
            });
            
            document.getElementById('unifiedBtn').addEventListener('click', () => {
                setDiffMode('unified');
            });

            loadDiffResult();
        });

        async function loadDiffResult() {
            try {
                const response = await fetch('/api/diff');
                const result = await response.json();
                
                if (result.success) {
                    diffResult = result.data;
                    renderFileTree(result.data.tree);
                    updateStats(result.data);
                } else {
                    document.getElementById('fileTree').innerHTML = 
                        `<div class="error">Error: ${result.error}</div>`;
                }
            } catch (error) {
                document.getElementById('fileTree').innerHTML = 
                    `<div class="error">Failed to load diff result</div>`;
                console.error('Error loading diff result:', error);
            }
        }

        function renderFileTree(tree, level = 0) {
            const container = document.getElementById('fileTree');
            container.innerHTML = '';
            renderTreeNode(tree, container, level);
        }

        function renderTreeNode(node, container, level) {
            // Only show the item if it has a path (skip the root empty node)
            if (node.relative_path && node.relative_path !== '') {
                const item = document.createElement('div');
                item.className = 'file-item';
                item.style.paddingLeft = `${level * 16 + 8}px`;
                
                const statusIcon = document.createElement('span');
                statusIcon.className = `status-icon status-${node.status.toLowerCase()}`;
                statusIcon.textContent = getStatusIcon(node.status);
                
                // Add tree connector symbols
                const treeConnector = document.createElement('span');
                treeConnector.className = 'tree-connector';
                treeConnector.textContent = level > 0 ? '├─ ' : '';
                
                const fileIcon = document.createElement('span');
                fileIcon.className = 'file-icon';
                fileIcon.textContent = node.is_directory ? '📁' : '📄';
                
                const fileName = document.createElement('span');
                const pathParts = node.relative_path.split(/[/\\]/);
                fileName.textContent = pathParts[pathParts.length - 1];
                
                item.appendChild(statusIcon);
                item.appendChild(treeConnector);
                item.appendChild(fileIcon);
                item.appendChild(fileName);
                
                if (!node.is_directory) {
                    item.addEventListener('click', () => selectFile(node.relative_path, fileName.textContent));
                }
                
                container.appendChild(item);
            }
            
            // Render children with proper indentation
            if (node.children && node.children.length > 0) {
                // Sort children: directories first, then files
                const sortedChildren = [...node.children].sort((a, b) => {
                    if (a.is_directory && !b.is_directory) return -1;
                    if (!a.is_directory && b.is_directory) return 1;
                    return a.relative_path.localeCompare(b.relative_path);
                });
                
                sortedChildren.forEach(child => {
                    renderTreeNode(child, container, node.relative_path === '' ? level : level + 1);
                });
            }
        }

        function getStatusIcon(status) {
            switch (status.toLowerCase()) {
                case 'added': return '+';
                case 'removed': return '-';
                case 'modified': return '~';
                case 'unchanged': return ' ';
                case 'conflicted': return '!';
                default: return ' ';
            }
        }

        function setDiffMode(mode) {
            diffMode = mode;
            
            // Update button states
            document.querySelectorAll('.mode-btn').forEach(btn => btn.classList.remove('active'));
            if (mode === 'side-by-side') {
                document.getElementById('sideBySideBtn').classList.add('active');
                diffEditor.updateOptions({ renderSideBySide: true });
            } else {
                document.getElementById('unifiedBtn').classList.add('active');
                diffEditor.updateOptions({ renderSideBySide: false });
            }
            
            // Refresh the current diff if one is loaded
            if (currentDiff) {
                displayDiff(currentDiff.diff, currentDiff.fileName);
            }
        }

        async function selectFile(filePath, fileName) {
            document.querySelectorAll('.file-item').forEach(item => {
                item.classList.remove('selected');
            });
            event.target.closest('.file-item').classList.add('selected');
            
            document.getElementById('currentFile').textContent = fileName;
            
            try {
                const response = await fetch(`/api/file?path=${encodeURIComponent(filePath)}`);
                const result = await response.json();
                
                if (result.success) {
                    currentDiff = { diff: result.data, fileName };
                    displayDiff(result.data, fileName);
                } else {
                    const errorModel = monaco.editor.createModel(`Error: ${result.error}`, 'text');
                    diffEditor.setModel({
                        original: errorModel,
                        modified: errorModel
                    });
                }
            } catch (error) {
                const errorModel = monaco.editor.createModel('Error loading file content', 'text');
                diffEditor.setModel({
                    original: errorModel,
                    modified: errorModel
                });
                console.error('Error loading file diff:', error);
            }
        }

        function displayDiff(diff, fileName) {
            const leftContent = diff.left_content || '';
            const rightContent = diff.right_content || '';
            
            // Determine file language from extension for syntax highlighting
            const language = getLanguageFromFileName(fileName);
            
            // Ensure we always have the diff editor
            if (!diffEditor || document.getElementById('unifiedEditor')) {
                document.getElementById('diffEditor').innerHTML = '';
                diffEditor = monaco.editor.createDiffEditor(document.getElementById('diffEditor'), {
                    theme: 'amoled-dark',
                    readOnly: true,
                    automaticLayout: true,
                    minimap: { enabled: false },
                    scrollBeyondLastLine: false,
                    fontSize: 13,
                    lineNumbers: 'on',
                    renderWhitespace: 'selection',
                    wordWrap: 'off',
                    renderSideBySide: diffMode === 'side-by-side',
                    ignoreTrimWhitespace: false,
                    renderIndicators: true
                });
            }
            
            // Update the render mode
            diffEditor.updateOptions({ 
                renderSideBySide: diffMode === 'side-by-side' 
            });
            
            // Create models with appropriate language
            const originalModel = monaco.editor.createModel(leftContent, language);
            const modifiedModel = monaco.editor.createModel(rightContent, language);
            
            diffEditor.setModel({
                original: originalModel,
                modified: modifiedModel
            });
        }


        function getLanguageFromFileName(fileName) {
            const ext = fileName.split('.').pop().toLowerCase();
            const languageMap = {
                'js': 'javascript',
                'jsx': 'javascript',
                'ts': 'typescript',
                'tsx': 'typescript',
                'py': 'python',
                'rs': 'rust',
                'go': 'go',
                'java': 'java',
                'c': 'c',
                'cpp': 'cpp',
                'h': 'c',
                'hpp': 'cpp',
                'css': 'css',
                'html': 'html',
                'xml': 'xml',
                'json': 'json',
                'yaml': 'yaml',
                'yml': 'yaml',
                'md': 'markdown',
                'sh': 'shell',
                'bash': 'shell',
                'sql': 'sql',
                'php': 'php',
                'rb': 'ruby',
                'swift': 'swift',
                'kt': 'kotlin'
            };
            return languageMap[ext] || 'text';
        }

        function updateStats(diffResult) {
            const stats = document.getElementById('stats');
            stats.innerHTML = `
                <div class="stat-item">
                    <span class="status-icon status-added">+</span>
                    <span>${diffResult.added_count}</span>
                </div>
                <div class="stat-item">
                    <span class="status-icon status-removed">-</span>
                    <span>${diffResult.removed_count}</span>
                </div>
                <div class="stat-item">
                    <span class="status-icon status-modified">~</span>
                    <span>${diffResult.modified_count}</span>
                </div>
                <div class="stat-item">
                    <span>Total: ${diffResult.total_files}</span>
                </div>
            `;
        }
    </script>
</body>  
</html>"#;

pub async fn start_server(core: DiffyCore, port: u16) -> Result<()> {
    let app = create_app(core);
    
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    
    println!("🚀 Diffy web server running at http://127.0.0.1:{}", port);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
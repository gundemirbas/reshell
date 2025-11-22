// WebSocket connection
let ws = null;
let reconnectAttempts = 0;
const maxReconnectAttempts = 5;

// DOM elements
const output = document.getElementById('output');
const commandInput = document.getElementById('command-input');
const sendBtn = document.getElementById('send-btn');
const clearBtn = document.getElementById('clear-btn');
const statusIndicator = document.getElementById('status-indicator');
const statusText = document.getElementById('status-text');
const terminal = document.getElementById('terminal');

// Connect to WebSocket server
function connect() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;
    
    addOutput(`\n[INFO] Connecting to ${wsUrl}...\n`, 'info');
    
    try {
        ws = new WebSocket(wsUrl);
        
        ws.onopen = () => {
            addOutput('[INFO] Connected to terminal!\n', 'info');
            updateStatus(true);
            reconnectAttempts = 0;
            commandInput.disabled = false;
            sendBtn.disabled = false;
            commandInput.focus();
        };
        
        ws.onmessage = (event) => {
            addOutput(event.data);
            scrollToBottom();
        };
        
        ws.onerror = (error) => {
            addOutput(`[ERROR] WebSocket error\n`, 'error');
            console.error('WebSocket error:', error);
        };
        
        ws.onclose = () => {
            addOutput('[INFO] Connection closed\n', 'info');
            updateStatus(false);
            commandInput.disabled = true;
            sendBtn.disabled = true;
            
            // Attempt reconnection
            if (reconnectAttempts < maxReconnectAttempts) {
                reconnectAttempts++;
                const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
                addOutput(`[INFO] Reconnecting in ${delay/1000}s... (attempt ${reconnectAttempts}/${maxReconnectAttempts})\n`, 'info');
                setTimeout(connect, delay);
            } else {
                addOutput('[ERROR] Max reconnection attempts reached. Please refresh the page.\n', 'error');
            }
        };
    } catch (error) {
        addOutput(`[ERROR] Failed to create WebSocket: ${error.message}\n`, 'error');
        updateStatus(false);
    }
}

// Send command
function sendCommand() {
    const command = commandInput.value.trim();
    if (!command || !ws || ws.readyState !== WebSocket.OPEN) {
        return;
    }
    
    // Send command with newline
    ws.send(command + '\n');
    commandInput.value = '';
}

// Add output to terminal
function addOutput(text, className = '') {
    const span = document.createElement('span');
    if (className) {
        span.className = className;
    }
    span.textContent = text;
    output.appendChild(span);
}

// Clear terminal
function clearTerminal() {
    output.innerHTML = '';
}

// Update connection status
function updateStatus(connected) {
    if (connected) {
        statusIndicator.classList.add('connected');
        statusText.textContent = 'Connected';
    } else {
        statusIndicator.classList.remove('connected');
        statusText.textContent = 'Disconnected';
    }
}

// Scroll terminal to bottom
function scrollToBottom() {
    terminal.scrollTop = terminal.scrollHeight;
}

// Event listeners
commandInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        sendCommand();
    }
});

sendBtn.addEventListener('click', sendCommand);
clearBtn.addEventListener('click', clearTerminal);

// Handle special keys
commandInput.addEventListener('keydown', (e) => {
    // Ctrl+C
    if (e.ctrlKey && e.key === 'c') {
        if (ws && ws.readyState === WebSocket.OPEN) {
            ws.send('\x03'); // Send Ctrl+C
            e.preventDefault();
        }
    }
    // Ctrl+D
    else if (e.ctrlKey && e.key === 'd') {
        if (ws && ws.readyState === WebSocket.OPEN) {
            ws.send('\x04'); // Send Ctrl+D (EOF)
            e.preventDefault();
        }
    }
});

// Start connection when page loads
window.addEventListener('load', () => {
    connect();
});

// Close connection when page unloads
window.addEventListener('beforeunload', () => {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.close();
    }
});

// WebSocket connection
let ws = null;
let reconnectAttempts = 0;
const maxReconnectAttempts = 5;

// DOM elements
const output = document.getElementById('output');
const clearBtn = document.getElementById('clear-btn');
const statusIndicator = document.getElementById('status-indicator');
const statusText = document.getElementById('status-text');
const terminal = document.getElementById('terminal');

// Connect to WebSocket server
function connect() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;
    
    addOutput(`[INFO] Connecting to ${wsUrl}...\n`, 'info');
    
    try {
        ws = new WebSocket(wsUrl);
        
        ws.onopen = () => {
            addOutput('[INFO] Connected to your personal shell!\n', 'info');
            addOutput('[INFO] Each browser has its own isolated session\n', 'info');
            addOutput('[INFO] Commands: pwd, ls, cd, echo, env, threads\n', 'info');
            addOutput('[INFO] Press Enter to execute\n\n', 'info');
            updateStatus(true);
            reconnectAttempts = 0;
            terminal.focus();
        };
        
        ws.onmessage = (event) => {
            addOutput(event.data);
            scrollToBottom();
        };
        
        ws.onerror = (error) => {
            addOutput('[ERROR] WebSocket error\n', 'error');
            console.error('WebSocket error:', error);
        };
        
        ws.onclose = () => {
            addOutput('[INFO] Connection closed\n', 'info');
            updateStatus(false);
            
            if (reconnectAttempts < maxReconnectAttempts) {
                reconnectAttempts++;
                addOutput(`[INFO] Reconnecting (${reconnectAttempts}/${maxReconnectAttempts})...\n`, 'info');
                setTimeout(connect, 2000);
            } else {
                addOutput('[ERROR] Max reconnection attempts reached\n', 'error');
            }
        };
    } catch (error) {
        addOutput(`[ERROR] Failed: ${error.message}\n`, 'error');
        updateStatus(false);
    }
}

// Send single character to server
function sendChar(char) {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(char);
    }
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
    addOutput('[INFO] Terminal cleared\n', 'info');
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

// Keypress handler with local echo
document.addEventListener('keydown', (e) => {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
        return;
    }
    
    // Allow F5, F12, Ctrl+R for refresh
    if (e.key === 'F5' || e.key === 'F12' || (e.ctrlKey && e.key === 'r')) {
        return;
    }
    
    // Prevent default for most keys
    e.preventDefault();
    
    // Handle special keys - Server echoes everything, no local echo needed
    if (e.key === 'Enter') {
        sendChar('\n');
    } else if (e.key === 'Backspace') {
        sendChar('\x7f');
    } else if (e.key === 'Tab') {
        sendChar('\t');
    } else if (e.ctrlKey && e.key === 'c') {
        sendChar('\x03');
    } else if (e.ctrlKey && e.key === 'd') {
        sendChar('\x04');
    } else if (e.key.length === 1) {
        // Send character to server - server will echo it back
        sendChar(e.key);
    }
    
    scrollToBottom();
});

// Click to focus terminal
terminal.addEventListener('click', () => {
    terminal.focus();
});

// Clear button
clearBtn.addEventListener('click', clearTerminal);

// Make terminal focusable
terminal.setAttribute('tabindex', '0');

// Connect on load
connect();

// Focus terminal on page load
window.addEventListener('load', () => {
    setTimeout(() => terminal.focus(), 100);
});

const NATIVE_HOST_NAME = 'com.urshell.host';

let currentUrl = '';
let commands = [];
let port = null;

document.addEventListener('DOMContentLoaded', async () => {
  // Get current tab URL
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  currentUrl = tab.url;

  // Settings button handlers
  document.getElementById('settings-btn').addEventListener('click', openSettings);
  document.getElementById('open-settings-btn').addEventListener('click', openSettings);

  // Cancel button handler
  document.getElementById('cancel-btn').addEventListener('click', cancelCommand);

  // Add click-to-copy for command codes
  document.querySelectorAll('.platform .command').forEach(el => {
    el.addEventListener('click', async () => {
      await navigator.clipboard.writeText(el.textContent);
      el.classList.add('copied');
      setTimeout(() => el.classList.remove('copied'), 1000);
    });
  });

  // Fetch commands from native host
  fetchCommands();
});

function openSettings() {
  chrome.runtime.openOptionsPage();
  window.close();
}

function fetchCommands() {
  try {
    port = chrome.runtime.connectNative(NATIVE_HOST_NAME);

    port.onMessage.addListener((response) => {
      if (response.status === 'commands') {
        commands = response.commands;
        handleCommandsReceived();
      } else if (response.status === 'error') {
        showError(response.message);
      } else {
        // Handle run responses
        handleRunResponse(response);
      }
    });

    port.onDisconnect.addListener(() => {
      const error = chrome.runtime.lastError;
      if (error && commands.length === 0) {
        // Check if native host is not installed
        if (error.message && error.message.includes('not found')) {
          showSetup();
        } else {
          showError(`Connection failed: ${error.message}`);
        }
      }
      port = null;
    });

    // Request command list
    port.postMessage({ action: 'get_commands' });

  } catch (error) {
    showError(`Failed to connect: ${error.message}`);
  }
}

function handleCommandsReceived() {
  document.getElementById('loading').classList.add('hidden');

  if (commands.length === 0) {
    // No commands configured - show error with settings button
    showError('No commands configured');
  } else if (commands.length === 1) {
    // Single command: execute immediately
    showStatus('running', `Running: ${commands[0].name}`);
    runCommand(0);
  } else {
    // Multiple commands: show picker
    showCommandList();
  }
}

function showCommandList() {
  const commandList = document.getElementById('command-list');
  const commandsContainer = document.getElementById('commands');
  const urlDisplay = document.getElementById('current-url');

  urlDisplay.textContent = currentUrl;

  // Create a button for each command
  commandsContainer.innerHTML = '';
  commands.forEach((cmd, index) => {
    const btn = document.createElement('button');
    btn.className = 'command-btn';
    btn.textContent = cmd.name;
    btn.addEventListener('click', () => {
      // Disable all buttons
      document.querySelectorAll('.command-btn').forEach(b => b.disabled = true);
      btn.textContent = 'Running...';
      runCommand(index);
    });
    commandsContainer.appendChild(btn);
  });

  commandList.classList.remove('hidden');
}

function runCommand(index) {
  // Reconnect if needed
  if (!port) {
    port = chrome.runtime.connectNative(NATIVE_HOST_NAME);
    port.onMessage.addListener((response) => {
      if (response.status !== 'commands') {
        handleRunResponse(response);
      }
    });
    port.onDisconnect.addListener(() => {
      port = null;
    });
  }

  showStatus('running', `Running: ${commands[index].name}`);

  port.postMessage({
    action: 'run',
    url: currentUrl,
    command_index: index
  });
}

function handleRunResponse(response) {
  switch (response.status) {
    case 'started':
      // Already showing running status
      break;

    case 'output':
      updateOutput(response.data);
      break;

    case 'complete':
      showStatus('complete', 'Complete', response.output);
      if (port) {
        port.disconnect();
        port = null;
      }
      break;

    case 'cancelled':
      showStatus('cancelled', 'Cancelled');
      if (port) {
        port.disconnect();
        port = null;
      }
      break;

    case 'error':
      showStatus('error', response.message || 'Command failed', response.output);
      if (port) {
        port.disconnect();
        port = null;
      }
      break;
  }
}

function cancelCommand() {
  if (port) {
    port.postMessage({ action: 'cancel' });
  }
}

function showStatus(status, message, output = '') {
  document.getElementById('loading').classList.add('hidden');
  document.getElementById('command-list').classList.add('hidden');
  document.getElementById('error-section').classList.add('hidden');

  const statusSection = document.getElementById('status-section');
  const statusMessage = document.getElementById('status-message');
  const outputEl = document.getElementById('output');
  const cancelBtn = document.getElementById('cancel-btn');

  statusSection.classList.remove('hidden');
  statusSection.className = 'status-section status-' + status;
  statusMessage.textContent = message;

  // Show cancel button only while running
  if (status === 'running') {
    cancelBtn.classList.remove('hidden');
  } else {
    cancelBtn.classList.add('hidden');
  }

  if (output) {
    const lines = output.split('\n').slice(-15).join('\n');
    outputEl.textContent = lines;
  }
}

function updateOutput(data) {
  const outputEl = document.getElementById('output');
  const current = outputEl.textContent;
  const lines = (current + '\n' + data).split('\n').slice(-15).join('\n');
  outputEl.textContent = lines;
}

function showError(message) {
  document.getElementById('loading').classList.add('hidden');
  document.getElementById('command-list').classList.add('hidden');
  document.getElementById('status-section').classList.add('hidden');
  document.getElementById('setup-section').classList.add('hidden');

  const errorSection = document.getElementById('error-section');
  const errorMessage = document.getElementById('error-message');

  errorMessage.textContent = message;
  errorSection.classList.remove('hidden');
}

function showSetup() {
  document.getElementById('loading').classList.add('hidden');
  document.getElementById('command-list').classList.add('hidden');
  document.getElementById('status-section').classList.add('hidden');
  document.getElementById('error-section').classList.add('hidden');

  document.getElementById('setup-section').classList.remove('hidden');
}

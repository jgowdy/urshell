let currentUrl = '';
let currentTabId = null;
let commands = [];

document.addEventListener('DOMContentLoaded', async () => {
  // Get current tab URL and ID
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  currentUrl = tab.url;
  currentTabId = tab.id;

  // Settings button handlers
  document.getElementById('settings-btn').addEventListener('click', openSettings);
  document.getElementById('open-settings-btn').addEventListener('click', openSettings);

  // Cancel button handler
  document.getElementById('cancel-btn').addEventListener('click', cancelCommand);

  // Dismiss button handler
  document.getElementById('dismiss-btn').addEventListener('click', dismissStatus);

  // Add click-to-copy for command codes
  document.querySelectorAll('.platform .command').forEach(el => {
    el.addEventListener('click', async () => {
      await navigator.clipboard.writeText(el.textContent);
      el.classList.add('copied');
      setTimeout(() => el.classList.remove('copied'), 1000);
    });
  });

  // Listen for state updates from background
  chrome.runtime.onMessage.addListener((message) => {
    if (message.type === 'stateUpdate' && message.tabId === currentTabId) {
      handleStateUpdate(message.state);
    }
  });

  // Get current state from background
  chrome.runtime.sendMessage({ action: 'getState', tabId: currentTabId }, (response) => {
    if (response && response.state) {
      if (response.state.status !== 'idle') {
        // Show current running/completed state
        handleStateUpdate(response.state);
        if (response.commands) {
          commands = response.commands;
        }
        return;
      }
    }
    // No active execution, fetch commands
    fetchCommands();
  });
});

function openSettings() {
  chrome.runtime.openOptionsPage();
  window.close();
}

function fetchCommands() {
  chrome.runtime.sendMessage({ action: 'getCommands', tabId: currentTabId }, (response) => {
    if (chrome.runtime.lastError) {
      showError(`Connection failed: ${chrome.runtime.lastError.message}`);
      return;
    }

    if (response && response.commands) {
      commands = response.commands;
      handleCommandsReceived();
    } else {
      showSetup();
    }
  });
}

function handleCommandsReceived() {
  document.getElementById('loading').classList.add('hidden');

  if (commands.length === 0) {
    showError('No commands configured');
  } else if (commands.length === 1) {
    // Single command: execute immediately
    showStatus('running', `Running: ${commands[0].name}`);
    runCommand(0);
  } else {
    showCommandList();
  }
}

function showCommandList() {
  const commandList = document.getElementById('command-list');
  const commandsContainer = document.getElementById('commands');
  const urlDisplay = document.getElementById('current-url');

  urlDisplay.textContent = currentUrl;

  commandsContainer.innerHTML = '';
  commands.forEach((cmd, index) => {
    const btn = document.createElement('button');
    btn.className = 'command-btn';
    btn.textContent = cmd.name;
    btn.addEventListener('click', () => {
      document.querySelectorAll('.command-btn').forEach(b => b.disabled = true);
      btn.textContent = 'Running...';
      runCommand(index);
    });
    commandsContainer.appendChild(btn);
  });

  commandList.classList.remove('hidden');
}

function runCommand(index) {
  showStatus('running', `Running: ${commands[index].name}`);

  chrome.runtime.sendMessage({
    action: 'run',
    tabId: currentTabId,
    url: currentUrl,
    commandIndex: index,
    commandName: commands[index].name
  });
}

function cancelCommand() {
  chrome.runtime.sendMessage({ action: 'cancel', tabId: currentTabId });
}

function dismissStatus() {
  chrome.runtime.sendMessage({ action: 'reset', tabId: currentTabId });
  window.close();
}

function handleStateUpdate(state) {
  switch (state.status) {
    case 'idle':
      // Nothing running, show command list
      fetchCommands();
      break;

    case 'running':
      showStatus('running', `Running: ${state.commandName}`, state.output);
      break;

    case 'complete':
      showStatus('complete', 'Complete', state.output);
      break;

    case 'cancelled':
      showStatus('cancelled', 'Cancelled', state.output);
      break;

    case 'error':
      showStatus('error', state.message || 'Command failed', state.output);
      break;
  }
}

function showStatus(status, message, output = '') {
  document.getElementById('loading').classList.add('hidden');
  document.getElementById('command-list').classList.add('hidden');
  document.getElementById('error-section').classList.add('hidden');
  document.getElementById('setup-section').classList.add('hidden');

  const statusSection = document.getElementById('status-section');
  const statusMessage = document.getElementById('status-message');
  const outputEl = document.getElementById('output');
  const cancelBtn = document.getElementById('cancel-btn');
  const dismissBtn = document.getElementById('dismiss-btn');

  statusSection.classList.remove('hidden');
  statusSection.className = 'status-section status-' + status;
  statusMessage.textContent = message;

  if (status === 'running') {
    cancelBtn.classList.remove('hidden');
    dismissBtn.classList.add('hidden');
  } else {
    cancelBtn.classList.add('hidden');
    dismissBtn.classList.remove('hidden');
  }

  if (output) {
    const lines = output.split('\n').slice(-15).join('\n');
    outputEl.textContent = lines;
  } else {
    outputEl.textContent = '';
  }
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

const NATIVE_HOST_NAME = 'com.urshell.host';

// Per-tab execution state: tabId -> { status, commandName, output, message, port }
const tabStates = new Map();

let cachedCommands = null;

function getTabState(tabId) {
  if (!tabStates.has(tabId)) {
    tabStates.set(tabId, {
      status: 'idle',
      commandName: '',
      output: '',
      message: '',
      port: null
    });
  }
  return tabStates.get(tabId);
}

// Clean up state when tab is closed
chrome.tabs.onRemoved.addListener((tabId) => {
  const state = tabStates.get(tabId);
  if (state && state.port) {
    state.port.disconnect();
  }
  tabStates.delete(tabId);
});

// Handle keyboard shortcut
chrome.commands.onCommand.addListener(async (command) => {
  if (command === 'run-command') {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tab && tab.url) {
      getCommands((commands) => {
        if (commands && commands.length > 0) {
          runCommand(tab.id, tab.url, 0, commands[0].name);
        }
      });
    }
  }
});

// Handle messages from popup
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  const tabId = request.tabId;

  switch (request.action) {
    case 'getState':
      const state = getTabState(tabId);
      sendResponse({
        state: {
          status: state.status,
          commandName: state.commandName,
          output: state.output,
          message: state.message
        },
        commands: cachedCommands
      });
      break;

    case 'getCommands':
      getCommands((commands) => {
        sendResponse({ commands });
      });
      return true; // Will respond async

    case 'run':
      runCommand(tabId, request.url, request.commandIndex, request.commandName);
      sendResponse({ ok: true });
      break;

    case 'cancel':
      cancelCommand(tabId);
      sendResponse({ ok: true });
      break;
  }
});

function broadcastState(tabId) {
  const state = getTabState(tabId);
  chrome.runtime.sendMessage({
    type: 'stateUpdate',
    tabId: tabId,
    state: {
      status: state.status,
      commandName: state.commandName,
      output: state.output,
      message: state.message
    }
  }).catch(() => {
    // Popup might not be open, ignore
  });
}

function setState(tabId, updates) {
  const state = getTabState(tabId);
  Object.assign(state, updates);
  broadcastState(tabId);
}

function getCommands(callback) {
  const port = chrome.runtime.connectNative(NATIVE_HOST_NAME);

  port.onMessage.addListener((response) => {
    if (response.status === 'commands') {
      cachedCommands = response.commands;
      callback(response.commands);
      port.disconnect();
    }
  });

  port.onDisconnect.addListener(() => {
    const error = chrome.runtime.lastError;
    if (error) {
      callback(null, error.message);
    }
  });

  port.postMessage({ action: 'get_commands' });
}

function runCommand(tabId, url, commandIndex, commandName) {
  const state = getTabState(tabId);

  // If already running, don't start another
  if (state.status === 'running') {
    return;
  }

  // Reset state
  setState(tabId, {
    status: 'running',
    commandName: commandName,
    output: '',
    message: ''
  });

  const nativePort = chrome.runtime.connectNative(NATIVE_HOST_NAME);
  state.port = nativePort;

  nativePort.onMessage.addListener((response) => {
    // Make sure this tab still exists
    if (!tabStates.has(tabId)) {
      nativePort.disconnect();
      return;
    }

    switch (response.status) {
      case 'started':
        // Already set to running
        break;

      case 'output':
        const currentState = getTabState(tabId);
        const lines = (currentState.output + '\n' + response.data).split('\n').slice(-15).join('\n');
        setState(tabId, { output: lines });
        break;

      case 'complete':
        setState(tabId, {
          status: 'complete',
          output: response.output || getTabState(tabId).output
        });
        state.port = null;
        nativePort.disconnect();
        break;

      case 'cancelled':
        setState(tabId, { status: 'cancelled' });
        state.port = null;
        nativePort.disconnect();
        break;

      case 'error':
        setState(tabId, {
          status: 'error',
          message: response.message || 'Command failed',
          output: response.output || getTabState(tabId).output
        });
        state.port = null;
        nativePort.disconnect();
        break;
    }
  });

  nativePort.onDisconnect.addListener(() => {
    const error = chrome.runtime.lastError;
    const currentState = tabStates.get(tabId);
    if (currentState && currentState.status === 'running') {
      setState(tabId, {
        status: 'error',
        message: error ? error.message : 'Disconnected'
      });
    }
    if (currentState) {
      currentState.port = null;
    }
  });

  nativePort.postMessage({
    action: 'run',
    url: url,
    command_index: commandIndex
  });
}

function cancelCommand(tabId) {
  const state = tabStates.get(tabId);
  if (state && state.port) {
    state.port.postMessage({ action: 'cancel' });
  }
}

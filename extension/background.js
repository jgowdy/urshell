const NATIVE_HOST_NAME = 'com.urshell.host';

// Handle keyboard shortcut - only works with single command config
chrome.commands.onCommand.addListener(async (command) => {
  if (command === 'run-command') {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tab && tab.url) {
      runFirstCommand(tab.url);
    }
  }
});

function runFirstCommand(url) {
  const port = chrome.runtime.connectNative(NATIVE_HOST_NAME);
  let commandName = 'Command';

  port.onMessage.addListener((response) => {
    if (response.status === 'commands') {
      // Got command list - run the first one
      if (response.commands && response.commands.length > 0) {
        commandName = response.commands[0].name;
        port.postMessage({
          action: 'run',
          url: url,
          command_index: 0
        });
      }
    } else if (response.status === 'complete') {
      chrome.notifications.create({
        type: 'basic',
        iconUrl: 'icons/icon128.png',
        title: 'URShell',
        message: `${commandName}: Complete`
      });
      port.disconnect();
    } else if (response.status === 'error') {
      chrome.notifications.create({
        type: 'basic',
        iconUrl: 'icons/icon128.png',
        title: 'URShell Error',
        message: response.message || 'Command failed'
      });
      port.disconnect();
    }
  });

  port.onDisconnect.addListener(() => {
    const error = chrome.runtime.lastError;
    if (error) {
      console.error('Native host disconnected:', error.message);
    }
  });

  // First get commands, then run
  port.postMessage({ action: 'get_commands' });
}

const NATIVE_HOST_NAME = 'com.urshell.host';

let commands = [];
let port = null;

document.addEventListener('DOMContentLoaded', () => {
  loadConfig();

  document.getElementById('add-cmd-btn').addEventListener('click', addCommand);
  document.getElementById('save-btn').addEventListener('click', saveConfig);
  document.getElementById('retry-btn').addEventListener('click', loadConfig);
});

function loadConfig() {
  showLoading();

  try {
    port = chrome.runtime.connectNative(NATIVE_HOST_NAME);

    port.onMessage.addListener((response) => {
      if (response.status === 'commands') {
        commands = response.commands || [];
        showConfigSection();
        renderCommands();
      } else if (response.status === 'saved') {
        showSaveStatus('Configuration saved!', 'success');
      } else if (response.status === 'error') {
        if (commands.length === 0) {
          showError(response.message);
        } else {
          showSaveStatus(response.message, 'error');
        }
      }
    });

    port.onDisconnect.addListener(() => {
      const error = chrome.runtime.lastError;
      if (error && commands.length === 0) {
        showError(`Connection failed: ${error.message}`);
      }
      port = null;
    });

    port.postMessage({ action: 'get_commands' });

  } catch (error) {
    showError(`Failed to connect: ${error.message}`);
  }
}

function showLoading() {
  document.getElementById('loading').classList.remove('hidden');
  document.getElementById('config-section').classList.add('hidden');
  document.getElementById('error-section').classList.add('hidden');
  document.getElementById('status').classList.add('hidden');
}

function showConfigSection() {
  document.getElementById('loading').classList.add('hidden');
  document.getElementById('config-section').classList.remove('hidden');
  document.getElementById('error-section').classList.add('hidden');
}

function showError(message) {
  document.getElementById('loading').classList.add('hidden');
  document.getElementById('config-section').classList.add('hidden');
  document.getElementById('error-section').classList.remove('hidden');
  document.getElementById('error-message').textContent = message;
}

function showSaveStatus(message, type) {
  const statusEl = document.getElementById('save-status');
  statusEl.textContent = message;
  statusEl.className = 'save-status ' + type;

  setTimeout(() => {
    statusEl.textContent = '';
    statusEl.className = 'save-status';
  }, 3000);
}

function renderCommands() {
  const container = document.getElementById('commands-list');
  container.innerHTML = '';

  if (commands.length === 0) {
    container.innerHTML = '<p class="no-commands">No commands configured. Add one to get started.</p>';
    return;
  }

  commands.forEach((cmd, index) => {
    const div = document.createElement('div');
    div.className = 'command-item';
    div.innerHTML = `
      <div class="command-fields">
        <div class="field">
          <label>Name</label>
          <input type="text" class="cmd-name" value="${escapeHtml(cmd.name)}" placeholder="Display name">
        </div>
        <div class="field field-wide">
          <label>Command</label>
          <input type="text" class="cmd-command" value="${escapeHtml(cmd.command)}" placeholder="Shell command (use % for URL position)">
        </div>
      </div>
      <div class="command-actions">
        <button class="btn btn-icon btn-move-up" title="Move up" ${index === 0 ? 'disabled' : ''}>&#9650;</button>
        <button class="btn btn-icon btn-move-down" title="Move down" ${index === commands.length - 1 ? 'disabled' : ''}>&#9660;</button>
        <button class="btn btn-icon btn-delete" title="Delete">&#10005;</button>
      </div>
    `;

    // Event listeners for this command
    div.querySelector('.cmd-name').addEventListener('input', (e) => {
      commands[index].name = e.target.value;
    });

    div.querySelector('.cmd-command').addEventListener('input', (e) => {
      commands[index].command = e.target.value;
    });

    div.querySelector('.btn-move-up').addEventListener('click', () => moveCommand(index, -1));
    div.querySelector('.btn-move-down').addEventListener('click', () => moveCommand(index, 1));
    div.querySelector('.btn-delete').addEventListener('click', () => deleteCommand(index));

    container.appendChild(div);
  });
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML.replace(/"/g, '&quot;');
}

function addCommand() {
  commands.push({ name: '', command: '' });
  renderCommands();

  // Focus the new command's name field
  const inputs = document.querySelectorAll('.cmd-name');
  if (inputs.length > 0) {
    inputs[inputs.length - 1].focus();
  }
}

function deleteCommand(index) {
  commands.splice(index, 1);
  renderCommands();
}

function moveCommand(index, direction) {
  const newIndex = index + direction;
  if (newIndex < 0 || newIndex >= commands.length) return;

  const temp = commands[index];
  commands[index] = commands[newIndex];
  commands[newIndex] = temp;
  renderCommands();
}

function saveConfig() {
  // Validate
  const validCommands = commands.filter(cmd => cmd.name.trim() && cmd.command.trim());

  if (validCommands.length === 0) {
    showSaveStatus('Add at least one command with name and command', 'error');
    return;
  }

  // Check for empty fields
  const hasEmpty = commands.some(cmd => !cmd.name.trim() || !cmd.command.trim());
  if (hasEmpty) {
    showSaveStatus('Please fill in all command fields or remove empty entries', 'error');
    return;
  }

  // Reconnect if needed
  if (!port) {
    port = chrome.runtime.connectNative(NATIVE_HOST_NAME);
    port.onMessage.addListener((response) => {
      if (response.status === 'saved') {
        showSaveStatus('Configuration saved!', 'success');
      } else if (response.status === 'error') {
        showSaveStatus(response.message, 'error');
      }
    });
    port.onDisconnect.addListener(() => {
      port = null;
    });
  }

  // Send save request
  port.postMessage({
    action: 'save_config',
    commands: commands
  });

  showSaveStatus('Saving...', '');
}

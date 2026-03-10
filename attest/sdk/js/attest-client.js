const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');
const { v4: uuidv4 } = require('uuid');

class AttestError extends Error {
  constructor(message, code = 'ATTEST_ERROR', details = null) {
    super(message);
    this.name = 'AttestError';
    this.code = code;
    this.details = details;
  }
}

class AttestCLIError extends AttestError {
  constructor(message, exitCode, stderr) {
    super(message, 'CLI_ERROR', { exitCode, stderr });
    this.name = 'AttestCLIError';
  }
}

class AttestNotFoundError extends AttestError {
  constructor(resourceType, id) {
    super(`${resourceType} not found: ${id}`, 'NOT_FOUND', { resourceType, id });
    this.name = 'AttestNotFoundError';
  }
}

class AttestConfigurationError extends AttestError {
  constructor(message) {
    super(message, 'CONFIGURATION_ERROR');
    this.name = 'AttestConfigurationError';
  }
}

const parseJSON = (output) => {
  try {
    return JSON.parse(output.trim());
  } catch {
    return output.trim();
  }
};

class AttestClient {
  constructor(options = {}) {
    this.cliPath = options.cliPath || process.env.ATTEST_CLI_PATH || 'attest';
    this.dataDir = options.dataDir || process.env.ATTEST_DATA_DIR || null;
    this.verbose = options.verbose || false;
  }

  async _execute(args, options = {}) {
    const baseArgs = [];
    if (this.dataDir) {
      baseArgs.push('--data-dir', this.dataDir);
    }
    if (options.format === 'json' || this.verbose) {
      baseArgs.push('--format', 'json');
    }

    const spawnArgs = [...baseArgs, ...args];
    const fullCommand = spawnArgs.join(' ');

    return new Promise((resolve, reject) => {
      if (this.verbose) {
        console.log(`[attest] ${fullCommand}`);
      }

      const child = spawn(this.cliPath, spawnArgs, {
        stdio: ['pipe', 'pipe', 'pipe'],
        env: { ...process.env }
      });

      let stdout = '';
      let stderr = '';

      child.stdout.on('data', (data) => {
        stdout += data.toString();
      });

      child.stderr.on('data', (data) => {
        stderr += data.toString();
      });

      child.on('error', (error) => {
        reject(new AttestConfigurationError(`Failed to execute attest CLI: ${error.message}`));
      });

      child.on('close', (code) => {
        if (code === 0) {
          if (options.parseJSON !== false) {
            resolve(parseJSON(stdout));
          } else {
            resolve(stdout.trim());
          }
        } else {
          reject(new AttestCLIError(`Attest CLI exited with code ${code}`, code, stderr));
        }
      });

      if (options.input) {
        child.stdin.write(options.input);
        child.stdin.end();
      }
    });
  }

  async version() {
    return this._execute(['version'], { parseJSON: true });
  }

  async status() {
    return this._execute(['status'], { parseJSON: true });
  }

  async agentCreate(name, options = {}) {
    if (!name || typeof name !== 'string') {
      throw new AttestError('Agent name is required', 'INVALID_ARGUMENT');
    }

    const args = ['agent', 'create', name];

    if (options.type) {
      args.push('--type', options.type);
    }
    if (options.metadata) {
      args.push('--metadata', JSON.stringify(options.metadata));
    }

    return this._execute(args, { parseJSON: true });
  }

  async agentList(options = {}) {
    const args = ['agent', 'list'];
    if (options.includeRevoked) {
      args.push('--include-revoked');
    }
    if (options.format) {
      args.push('--format', options.format);
    }
    return this._execute(args, { parseJSON: true });
  }

  async agentShow(agentId) {
    if (!agentId) {
      throw new AttestError('Agent ID is required', 'INVALID_ARGUMENT');
    }
    try {
      return await this._execute(['agent', 'show', agentId], { parseJSON: true });
    } catch (error) {
      if (error.code === 'CLI_ERROR' && error.details?.exitCode === 1) {
        throw new AttestNotFoundError('Agent', agentId);
      }
      throw error;
    }
  }

  async agentDelete(agentId) {
    if (!agentId) {
      throw new AttestError('Agent ID is required', 'INVALID_ARGUMENT');
    }
    try {
      return await this._execute(['agent', 'delete', agentId], { parseJSON: true });
    } catch (error) {
      if (error.code === 'CLI_ERROR' && error.details?.exitCode === 1) {
        throw new AttestNotFoundError('Agent', agentId);
      }
      throw error;
    }
  }

  async agentExport(agentId) {
    if (!agentId) {
      throw new AttestError('Agent ID is required', 'INVALID_ARGUMENT');
    }
    return this._execute(['agent', 'export', agentId], { parseJSON: true });
  }

  async agentImport(filepath) {
    if (!filepath) {
      throw new AttestError('Filepath is required', 'INVALID_ARGUMENT');
    }
    if (!fs.existsSync(filepath)) {
      throw new AttestError(`File not found: ${filepath}`, 'FILE_NOT_FOUND');
    }
    return this._execute(['agent', 'import', filepath], { parseJSON: true });
  }

  async intentCreate(goal, options = {}) {
    if (!goal || typeof goal !== 'string') {
      throw new AttestError('Intent goal is required', 'INVALID_ARGUMENT');
    }

    const args = ['intent', 'create', goal];

    if (options.constraints) {
      args.push('--constraints', JSON.stringify(options.constraints));
    }
    if (options.acceptanceCriteria) {
      args.push('--acceptance-criteria', JSON.stringify(options.acceptanceCriteria));
    }
    if (options.agentId) {
      args.push('--agent-id', options.agentId);
    }
    if (options.ticket) {
      args.push('--ticket', options.ticket);
    }

    return this._execute(args, { parseJSON: true });
  }

  async intentList(options = {}) {
    const args = ['intent', 'list'];
    if (options.status) {
      args.push('--status', options.status);
    }
    if (options.limit) {
      args.push('--limit', String(options.limit));
    }
    if (options.format) {
      args.push('--format', options.format);
    }
    return this._execute(args, { parseJSON: true });
  }

  async intentShow(intentId) {
    if (!intentId) {
      throw new AttestError('Intent ID is required', 'INVALID_ARGUMENT');
    }
    try {
      return await this._execute(['intent', 'show', intentId], { parseJSON: true });
    } catch (error) {
      if (error.code === 'CLI_ERROR' && error.details?.exitCode === 1) {
        throw new AttestNotFoundError('Intent', intentId);
      }
      throw error;
    }
  }

  async intentLinkAction(intentId, actionId) {
    if (!intentId || !actionId) {
      throw new AttestError('Intent ID and action ID are required', 'INVALID_ARGUMENT');
    }
    return this._execute(['intent', 'link', intentId, actionId], { parseJSON: true });
  }

  async attestAction(agentId, action, target, options = {}) {
    if (!agentId || !action || !target) {
      throw new AttestError('Agent ID, action, and target are required', 'INVALID_ARGUMENT');
    }

    const args = ['attest', action, target, '--agent-id', agentId];

    if (options.intentId) {
      args.push('--intent-id', options.intentId);
    }
    if (options.input) {
      args.push('--input', options.input);
    }
    if (options.sessionId) {
      args.push('--session-id', options.sessionId);
    }

    return this._execute(args, { parseJSON: true });
  }

  async attestList(options = {}) {
    const args = ['attest', 'list'];
    if (options.agentId) {
      args.push('--agent-id', options.agentId);
    }
    if (options.intentId) {
      args.push('--intent-id', options.intentId);
    }
    if (options.limit) {
      args.push('--limit', String(options.limit));
    }
    return this._execute(args, { parseJSON: true });
  }

  async attestShow(attestationId) {
    if (!attestationId) {
      throw new AttestError('Attestation ID is required', 'INVALID_ARGUMENT');
    }
    try {
      return await this._execute(['attest', 'show', attestationId], { parseJSON: true });
    } catch (error) {
      if (error.code === 'CLI_ERROR' && error.details?.exitCode === 1) {
        throw new AttestNotFoundError('Attestation', attestationId);
      }
      throw error;
    }
  }

  async attestExport(attestationId, options = {}) {
    if (!attestationId) {
      throw new AttestError('Attestation ID is required', 'INVALID_ARGUMENT');
    }

    const args = ['attest', 'export', attestationId];
    if (options.output) {
      args.push('--output', options.output);
    }
    return this._execute(args, { parseJSON: true });
  }

  async attestImport(filepath) {
    if (!filepath) {
      throw new AttestError('Filepath is required', 'INVALID_ARGUMENT');
    }
    if (!fs.existsSync(filepath)) {
      throw new AttestError(`File not found: ${filepath}`, 'FILE_NOT_FOUND');
    }
    return this._execute(['attest', 'import', filepath], { parseJSON: true });
  }

  async verifyAttestation(attestationId) {
    if (!attestationId) {
      throw new AttestError('Attestation ID is required', 'INVALID_ARGUMENT');
    }
    try {
      return await this._execute(['verify', attestationId], { parseJSON: true });
    } catch (error) {
      if (error.code === 'CLI_ERROR' && error.details?.exitCode === 1) {
        throw new AttestNotFoundError('Attestation', attestationId);
      }
      throw error;
    }
  }

  async execRun(command, options = {}) {
    if (!command || typeof command !== 'string') {
      throw new AttestError('Command is required', 'INVALID_ARGUMENT');
    }

    const args = ['exec', command];

    if (options.reversible) {
      args.push('--reversible');
    }
    if (options.agentId) {
      args.push('--agent-id', options.agentId);
    }
    if (options.intentId) {
      args.push('--intent-id', options.intentId);
    }
    if (options.backupType) {
      args.push('--backup-type', options.backupType);
    }
    if (options.dryRun) {
      args.push('--dry-run');
    }
    if (options.sessionId) {
      args.push('--session-id', options.sessionId);
    }

    return this._execute(args, { parseJSON: true });
  }

  async rollback(actionId = 'last') {
    const args = ['rollback', actionId];
    return this._execute(args, { parseJSON: true });
  }

  async execHistory(options = {}) {
    const args = ['exec', 'history'];
    if (options.pendingOnly) {
      args.push('--pending-only');
    }
    return this._execute(args, { parseJSON: true });
  }

  async policyCheck(action, target, options = {}) {
    if (!action || !target) {
      throw new AttestError('Action and target are required', 'INVALID_ARGUMENT');
    }

    const args = ['policy', 'check', action, target];
    if (options.agentId) {
      args.push('--agent-id', options.agentId);
    }
    if (options.intentId) {
      args.push('--intent-id', options.intentId);
    }

    return this._execute(args, { parseJSON: true });
  }

  async policyList() {
    return this._execute(['policy', 'list'], { parseJSON: true });
  }

  async policyAdd(filepath) {
    if (!filepath) {
      throw new AttestError('Filepath is required', 'INVALID_ARGUMENT');
    }
    if (!fs.existsSync(filepath)) {
      throw new AttestError(`File not found: ${filepath}`, 'FILE_NOT_FOUND');
    }
    return this._execute(['policy', 'add', filepath], { parseJSON: true });
  }

  async policyRemove(policyId) {
    if (!policyId) {
      throw new AttestError('Policy ID is required', 'INVALID_ARGUMENT');
    }
    return this._execute(['policy', 'remove', policyId], { parseJSON: true });
  }

  async init(dataDir = null) {
    const args = ['init'];
    if (dataDir) {
      args.push('--data-dir', dataDir);
    }
    return this._execute(args, { parseJSON: true });
  }
}

module.exports = {
  AttestClient,
  AttestError,
  AttestCLIError,
  AttestNotFoundError,
  AttestConfigurationError
};

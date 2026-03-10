const { AttestClient, AttestError, AttestCLIError, AttestNotFoundError, AttestConfigurationError } = require('./attest-client');

module.exports = {
  AttestClient,
  AttestError,
  AttestCLIError,
  AttestNotFoundError,
  AttestConfigurationError,
  createClient: (options) => new AttestClient(options)
};

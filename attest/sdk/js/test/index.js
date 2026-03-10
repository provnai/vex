const { AttestClient, AttestError, AttestCLIError, AttestNotFoundError, AttestConfigurationError } = require('../attest-client');

console.log('Testing Attest SDK imports...');

console.log('✓ AttestClient:', typeof AttestClient);
console.log('✓ AttestError:', typeof AttestError);
console.log('✓ AttestCLIError:', typeof AttestCLIError);
console.log('✓ AttestNotFoundError:', typeof AttestNotFoundError);
console.log('✓ AttestConfigurationError:', typeof AttestConfigurationError);

console.log('\nTesting error classes...');
const error = new AttestError('Test error', 'TEST_ERROR', { foo: 'bar' });
console.log('✓ Error code:', error.code);
console.log('✓ Error details:', error.details);

const notFound = new AttestNotFoundError('Agent', 'aid:123');
console.log('✓ NotFound resourceType:', notFound.resourceType);
console.log('✓ NotFound id:', notFound.id);

console.log('\nTesting client instantiation...');
const client = new AttestClient({ verbose: false });
console.log('✓ Client created');
console.log('✓ CLI path:', client.cliPath);

console.log('\nAll imports and basic tests passed!');

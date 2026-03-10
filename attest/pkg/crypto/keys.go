package crypto

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/ed25519"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"fmt"
	"io"

	"golang.org/x/crypto/pbkdf2"
)

// KeyPair represents a cryptographic key pair using Ed25519
type KeyPair struct {
	PublicKey  []byte
	PrivateKey []byte // Raw private key (must be encrypted before storage)
}

// GenerateEd25519KeyPair generates a new Ed25519 key pair
func GenerateEd25519KeyPair() (*KeyPair, error) {
	// Generate Ed25519 key pair using standard library
	_, privateKey, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		return nil, fmt.Errorf("failed to generate Ed25519 key pair: %w", err)
	}

	// Extract public key (last 32 bytes of private key in Ed25519)
	publicKey := privateKey.Public().(ed25519.PublicKey)

	return &KeyPair{
		PublicKey:  publicKey,
		PrivateKey: privateKey,
	}, nil
}

// Sign signs data with the private key using Ed25519
func (k *KeyPair) Sign(data []byte) ([]byte, error) {
	if len(k.PrivateKey) != ed25519.PrivateKeySize {
		return nil, fmt.Errorf("invalid private key size: expected %d, got %d", ed25519.PrivateKeySize, len(k.PrivateKey))
	}

	signature := ed25519.Sign(k.PrivateKey, data)
	return signature, nil
}

// Verify verifies a signature using Ed25519
func (k *KeyPair) Verify(data, signature []byte) bool {
	if len(k.PublicKey) != ed25519.PublicKeySize {
		return false
	}

	return ed25519.Verify(k.PublicKey, data, signature)
}

// PublicKeyBase64 returns the public key as base64
func (k *KeyPair) PublicKeyBase64() string {
	return base64.StdEncoding.EncodeToString(k.PublicKey)
}

// PrivateKeyHex returns the private key as hex (for storage - must be encrypted!)
func (k *KeyPair) PrivateKeyHex() string {
	return hex.EncodeToString(k.PrivateKey)
}

// AgentID generates a unique agent ID from public key hash
func (k *KeyPair) AgentID() string {
	hash := sha256.Sum256(k.PublicKey)
	return fmt.Sprintf("aid:%x", hash[:8])
}

// EncryptPrivateKey encrypts the private key using AES-256-GCM with PBKDF2
func (k *KeyPair) EncryptPrivateKey(passphrase string) (string, error) {
	if len(passphrase) < 8 {
		return "", fmt.Errorf("passphrase must be at least 8 characters")
	}

	// Generate a random salt (16 bytes)
	salt := make([]byte, 16)
	if _, err := rand.Read(salt); err != nil {
		return "", fmt.Errorf("failed to generate salt: %w", err)
	}

	// Derive key using PBKDF2 (100,000 iterations, SHA-256)
	dk, err := deriveKey(passphrase, salt, 32)
	if err != nil {
		return "", fmt.Errorf("failed to derive key: %w", err)
	}

	// Create AES-256-GCM cipher
	block, err := aes.NewCipher(dk)
	if err != nil {
		return "", fmt.Errorf("failed to create cipher: %w", err)
	}

	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return "", fmt.Errorf("failed to create GCM: %w", err)
	}

	// Generate nonce (12 bytes)
	nonce := make([]byte, gcm.NonceSize())
	if _, err := rand.Read(nonce); err != nil {
		return "", fmt.Errorf("failed to generate nonce: %w", err)
	}

	// Encrypt the private key
	ciphertext := gcm.Seal(nonce, nonce, k.PrivateKey, nil)

	// Combine salt + nonce + ciphertext for storage
	// Format: salt(16) + nonce(12) + ciphertext(variable)
	result := make([]byte, 16+12+len(ciphertext))
	copy(result[:16], salt)
	copy(result[16:28], nonce)
	copy(result[28:], ciphertext)

	return base64.StdEncoding.EncodeToString(result), nil
}

// DecryptPrivateKey decrypts a private key using AES-256-GCM with PBKDF2
func DecryptPrivateKey(encryptedBase64, passphrase string) ([]byte, error) {
	if len(passphrase) < 8 {
		return nil, fmt.Errorf("passphrase must be at least 8 characters")
	}

	// Decode from base64
	data, err := base64.StdEncoding.DecodeString(encryptedBase64)
	if err != nil {
		return nil, fmt.Errorf("failed to decode encrypted key: %w", err)
	}

	if len(data) < 28 {
		return nil, fmt.Errorf("invalid encrypted key format")
	}

	// Extract salt, nonce, and ciphertext
	salt := data[:16]
	nonce := data[16:28]
	ciphertext := data[28:]

	// Derive key using PBKDF2
	dk, err := deriveKey(passphrase, salt, 32)
	if err != nil {
		return nil, fmt.Errorf("failed to derive key: %w", err)
	}

	// Create AES-256-GCM cipher
	block, err := aes.NewCipher(dk)
	if err != nil {
		return nil, fmt.Errorf("failed to create cipher: %w", err)
	}

	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, fmt.Errorf("failed to create GCM: %w", err)
	}

	// Decrypt
	plaintext, err := gcm.Open(nil, nonce, ciphertext, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to decrypt: wrong passphrase or corrupted data")
	}

	return plaintext, nil
}

// deriveKey derives a key from passphrase using PBKDF2 with SHA-256
func deriveKey(passphrase string, salt []byte, keyLen int) ([]byte, error) {
	return pbkdf2.Key([]byte(passphrase), salt, 100000, keyLen, sha256.New), nil
}

// LoadKeyPair reconstructs a key pair from stored keys
func LoadKeyPair(publicKeyBase64, encryptedPrivateKeyHex, passphrase string) (*KeyPair, error) {
	publicKey, err := base64.StdEncoding.DecodeString(publicKeyBase64)
	if err != nil {
		return nil, fmt.Errorf("failed to decode public key: %w", err)
	}

	privateKey, err := DecryptPrivateKey(encryptedPrivateKeyHex, passphrase)
	if err != nil {
		return nil, fmt.Errorf("failed to decrypt private key: %w", err)
	}

	return &KeyPair{
		PublicKey:  publicKey,
		PrivateKey: privateKey,
	}, nil
}

// GenerateNonce generates a random nonce for encryption
func GenerateNonce() ([]byte, error) {
	nonce := make([]byte, 12)
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return nil, err
	}
	return nonce, nil
}

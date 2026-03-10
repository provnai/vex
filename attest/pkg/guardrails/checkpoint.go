package guardrails

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/provnai/attest/pkg/crypto"
)

// CheckpointManager handles checkpoint creation and management
type CheckpointManager struct {
	storageDir string
	maxSize    int64 // Maximum total size of checkpoints in bytes
}

// NewCheckpointManager creates a new checkpoint manager
func NewCheckpointManager(storageDir string) *CheckpointManager {
	return &CheckpointManager{
		storageDir: storageDir,
		maxSize:    1024 * 1024 * 1024, // 1GB default
	}
}

// SetMaxSize sets the maximum total storage for checkpoints
func (m *CheckpointManager) SetMaxSize(bytes int64) {
	m.maxSize = bytes
}

// CreateCheckpoint creates a checkpoint before a risky operation
func (m *CheckpointManager) CreateCheckpoint(ctx context.Context, op *Operation) (*Checkpoint, error) {
	checkpointID := generateCheckpointID(op)

	checkpoint := &Checkpoint{
		ID:          checkpointID,
		OperationID: op.ID,
		CreatedAt:   time.Now(),
		Type:        m.determineCheckpointType(op),
		Data:        make(map[string]interface{}),
	}

	// Create checkpoint directory
	checkpointDir := filepath.Join(m.storageDir, checkpointID)
	if err := os.MkdirAll(checkpointDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create checkpoint directory: %w", err)
	}

	// Capture file states if applicable
	if m.shouldCaptureFiles(op) {
		fileStates, err := m.captureFileStates(ctx, op)
		if err != nil {
			return nil, fmt.Errorf("failed to capture file states: %w", err)
		}
		checkpoint.FileStates = fileStates
	}

	// Capture DB states if applicable
	if m.shouldCaptureDB(op) {
		dbStates, err := m.captureDBStates(ctx, op)
		if err != nil {
			return nil, fmt.Errorf("failed to capture DB states: %w", err)
		}
		checkpoint.DBStates = dbStates
	}

	// Store environment state
	checkpoint.Data["env"] = op.Env
	checkpoint.Data["working_dir"] = op.WorkingDir
	checkpoint.Data["command"] = op.Command
	checkpoint.Data["args"] = op.Args

	// Save checkpoint metadata
	if err := m.saveCheckpoint(checkpoint, checkpointDir); err != nil {
		return nil, fmt.Errorf("failed to save checkpoint: %w", err)
	}

	// Calculate size
	size, err := m.calculateCheckpointSize(checkpointDir)
	if err != nil {
		return nil, fmt.Errorf("failed to calculate checkpoint size: %w", err)
	}
	checkpoint.Size = size

	// Cleanup old checkpoints if needed
	if err := m.cleanupOldCheckpoints(); err != nil {
		// Non-fatal: just log the error
		fmt.Printf("Warning: failed to cleanup old checkpoints: %v\n", err)
	}

	return checkpoint, nil
}

// GetCheckpoint retrieves a checkpoint by ID
func (m *CheckpointManager) GetCheckpoint(ctx context.Context, checkpointID string) (*Checkpoint, error) {
	checkpointPath := filepath.Join(m.storageDir, checkpointID, "checkpoint.json")

	data, err := os.ReadFile(checkpointPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, fmt.Errorf("checkpoint not found: %s", checkpointID)
		}
		return nil, fmt.Errorf("failed to read checkpoint: %w", err)
	}

	var checkpoint Checkpoint
	if err := json.Unmarshal(data, &checkpoint); err != nil {
		return nil, fmt.Errorf("failed to parse checkpoint: %w", err)
	}

	return &checkpoint, nil
}

// ListCheckpoints returns all available checkpoints
func (m *CheckpointManager) ListCheckpoints(ctx context.Context) ([]*Checkpoint, error) {
	entries, err := os.ReadDir(m.storageDir)
	if err != nil {
		if os.IsNotExist(err) {
			return []*Checkpoint{}, nil
		}
		return nil, fmt.Errorf("failed to list checkpoints: %w", err)
	}

	var checkpoints []*Checkpoint
	for _, entry := range entries {
		if entry.IsDir() {
			checkpoint, err := m.GetCheckpoint(ctx, entry.Name())
			if err == nil {
				checkpoints = append(checkpoints, checkpoint)
			}
		}
	}

	return checkpoints, nil
}

// DeleteCheckpoint removes a checkpoint
func (m *CheckpointManager) DeleteCheckpoint(ctx context.Context, checkpointID string) error {
	checkpointDir := filepath.Join(m.storageDir, checkpointID)
	return os.RemoveAll(checkpointDir)
}

// Rollback restores system state from a checkpoint
func (m *CheckpointManager) Rollback(ctx context.Context, checkpointID string) (*RollbackResult, error) {
	checkpoint, err := m.GetCheckpoint(ctx, checkpointID)
	if err != nil {
		return nil, err
	}

	result := &RollbackResult{
		CheckpointID: checkpointID,
		Success:      true,
	}

	start := time.Now()

	// Restore file states
	for _, fileState := range checkpoint.FileStates {
		if err := m.restoreFileState(ctx, fileState); err != nil {
			result.Errors = append(result.Errors, fmt.Errorf("failed to restore %s: %w", fileState.Path, err))
			result.Success = false
		} else {
			result.RestoredFiles++
		}
	}

	// Restore DB states
	for _, dbState := range checkpoint.DBStates {
		if err := m.restoreDBState(ctx, dbState); err != nil {
			result.Errors = append(result.Errors, fmt.Errorf("failed to restore DB table %s: %w", dbState.TableName, err))
			result.Success = false
		} else {
			result.RestoredDB++
		}
	}

	result.Duration = time.Since(start)

	return result, nil
}

// Helper methods

func (m *CheckpointManager) determineCheckpointType(op *Operation) string {
	// Determine checkpoint type based on operation
	cmd := strings.ToLower(op.Command)

	if strings.Contains(cmd, "rm") || strings.Contains(cmd, "del") {
		return "file_deletion"
	}
	if strings.Contains(cmd, "update") || strings.Contains(cmd, "modify") {
		return "modification"
	}
	if strings.Contains(cmd, "db") || strings.Contains(cmd, "database") {
		return "database"
	}
	if strings.Contains(cmd, "api") || strings.Contains(cmd, "curl") || strings.Contains(cmd, "wget") {
		return "api_call"
	}

	return "general"
}

func (m *CheckpointManager) shouldCaptureFiles(op *Operation) bool {
	cmd := strings.ToLower(op.Command)
	return strings.Contains(cmd, "rm") ||
		strings.Contains(cmd, "mv") ||
		strings.Contains(cmd, "cp") ||
		strings.Contains(cmd, "sed") ||
		strings.Contains(cmd, "edit")
}

func (m *CheckpointManager) shouldCaptureDB(op *Operation) bool {
	cmd := strings.ToLower(op.Command)
	return strings.Contains(cmd, "db") ||
		strings.Contains(cmd, "sql") ||
		strings.Contains(cmd, "migrate")
}

func (m *CheckpointManager) captureFileStates(ctx context.Context, op *Operation) ([]FileState, error) {
	// This is a simplified implementation
	// In a real system, you'd analyze the command to determine which files are affected
	var fileStates []FileState

	// For now, just capture the working directory state
	workingDir := op.WorkingDir
	if workingDir == "" {
		workingDir = "."
	}

	// Walk the directory and capture file hashes
	err := filepath.Walk(workingDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil // Skip files we can't access
		}

		if info.IsDir() {
			return nil
		}

		state := FileState{
			Path:        path,
			Exists:      true,
			Permissions: uint32(info.Mode()),
			ModTime:     info.ModTime(),
		}

		// Calculate file hash using streaming
		hash, err := crypto.HashFile(path)
		if err == nil {
			state.Hash = hash
		}

		// Only capture content if file is small (< 1MB)
		if info.Size() < 1024*1024 {
			if data, err := os.ReadFile(path); err == nil {
				state.Content = data
			}
		}

		fileStates = append(fileStates, state)
		return nil
	})

	return fileStates, err
}

func (m *CheckpointManager) captureDBStates(ctx context.Context, op *Operation) ([]DBState, error) {
	// Database state capture planned for v1.1
	return []DBState{}, nil
}

func (m *CheckpointManager) saveCheckpoint(checkpoint *Checkpoint, checkpointDir string) error {
	data, err := json.MarshalIndent(checkpoint, "", "  ")
	if err != nil {
		return err
	}

	checkpointPath := filepath.Join(checkpointDir, "checkpoint.json")
	return os.WriteFile(checkpointPath, data, 0644)
}

func (m *CheckpointManager) calculateCheckpointSize(checkpointDir string) (int64, error) {
	var size int64
	err := filepath.Walk(checkpointDir, func(_ string, info os.FileInfo, err error) error {
		if err != nil {
			return nil
		}
		if !info.IsDir() {
			size += info.Size()
		}
		return nil
	})
	return size, err
}

func (m *CheckpointManager) cleanupOldCheckpoints() error {
	// Get total size of all checkpoints
	var totalSize int64
	checkpoints, err := m.ListCheckpoints(context.Background())
	if err != nil {
		return err
	}

	for _, cp := range checkpoints {
		totalSize += cp.Size
	}

	// If over limit, delete oldest checkpoints
	if totalSize > m.maxSize {
		// Sort by creation time (oldest first)
		// For simplicity, just delete until under limit
		for _, cp := range checkpoints {
			if totalSize <= m.maxSize {
				break
			}
			if err := m.DeleteCheckpoint(context.Background(), cp.ID); err == nil {
				totalSize -= cp.Size
			}
		}
	}

	return nil
}

func (m *CheckpointManager) restoreFileState(ctx context.Context, state FileState) error {
	// Restore file from checkpoint
	if !state.Exists {
		// File was deleted, so delete it
		return os.Remove(state.Path)
	}

	// Ensure directory exists
	dir := filepath.Dir(state.Path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return err
	}

	// Restore file content
	if err := os.WriteFile(state.Path, state.Content, os.FileMode(state.Permissions)); err != nil {
		return err
	}

	// Restore modification time
	return os.Chtimes(state.Path, state.ModTime, state.ModTime)
}

func (m *CheckpointManager) restoreDBState(ctx context.Context, state DBState) error {
	// Database state restoration planned for v1.1
	return nil
}

func generateCheckpointID(op *Operation) string {
	// Generate a unique ID based on operation details and timestamp
	data := fmt.Sprintf("%s_%s_%d", op.Command, op.WorkingDir, time.Now().UnixNano())
	hash := sha256.Sum256([]byte(data))
	return "chk:" + hex.EncodeToString(hash[:])[:12]
}

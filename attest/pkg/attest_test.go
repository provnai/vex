package main

import (
	"os"
	"runtime"
	"testing"

	"github.com/provnai/attest/pkg/attestation"
	"github.com/provnai/attest/pkg/config"
	"github.com/provnai/attest/pkg/crypto"
	"github.com/provnai/attest/pkg/exec"
	"github.com/provnai/attest/pkg/identity"
	"github.com/provnai/attest/pkg/intent"
	"github.com/provnai/attest/pkg/policy"
	"github.com/provnai/attest/pkg/storage"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestConfigDefault(t *testing.T) {
	cfg := config.DefaultConfig()
	assert.NotNil(t, cfg)
	assert.Contains(t, cfg.DataDir, ".attest")
	assert.Equal(t, "info", cfg.LogLevel)
}

func TestConfigEnsureDirs(t *testing.T) {
	cfg := config.DefaultConfig()
	dir, err := os.MkdirTemp("", "attest-test-*")
	require.NoError(t, err)
	defer os.RemoveAll(dir)

	cfg.DataDir = dir + "/.attest"
	err = cfg.EnsureDirs()
	require.NoError(t, err)
	assert.DirExists(t, cfg.DataDir)
}

func TestCryptoGenerateKeyPair(t *testing.T) {
	keys, err := crypto.GenerateEd25519KeyPair()
	require.NoError(t, err)
	assert.NotNil(t, keys)
	assert.Len(t, keys.PublicKey, 32)
	assert.Len(t, keys.PrivateKey, 64)
}

func TestCryptoSignVerify(t *testing.T) {
	keys, err := crypto.GenerateEd25519KeyPair()
	require.NoError(t, err)

	data := []byte("test data")
	sig, err := keys.Sign(data)
	require.NoError(t, err)
	assert.NotEmpty(t, sig)

	valid := keys.Verify(data, sig)
	assert.True(t, valid)

	invalid := keys.Verify([]byte("wrong data"), sig)
	assert.False(t, invalid)
}

func TestCryptoAgentID(t *testing.T) {
	keys, err := crypto.GenerateEd25519KeyPair()
	require.NoError(t, err)

	agentID := keys.AgentID()
	assert.Contains(t, agentID, "aid:")
	assert.Len(t, agentID, 20) // "aid:" + 16 hex chars (8 bytes)
}

func TestCryptoPublicKeyBase64(t *testing.T) {
	keys, _ := crypto.GenerateEd25519KeyPair()
	b64 := keys.PublicKeyBase64()
	assert.NotEmpty(t, b64)
	assert.Greater(t, len(b64), 40)
}

func TestAgentCreation(t *testing.T) {
	keys, err := crypto.GenerateEd25519KeyPair()
	require.NoError(t, err)

	agent, err := identity.CreateAgent("test-agent", identity.AgentTypeGeneric, keys, identity.AgentMeta{})
	require.NoError(t, err)
	assert.NotNil(t, agent)
	assert.Equal(t, "test-agent", agent.Name)
	assert.Equal(t, identity.AgentTypeGeneric, agent.Type)
	assert.NotEmpty(t, agent.ID)
	assert.Contains(t, agent.ID, "aid:")
}

func TestAgentRevoke(t *testing.T) {
	keys, _ := crypto.GenerateEd25519KeyPair()
	agent, _ := identity.CreateAgent("test-agent", identity.AgentTypeGeneric, keys, identity.AgentMeta{})

	assert.False(t, agent.IsRevoked())
	agent.Revoke()
	assert.True(t, agent.IsRevoked())
}

func TestAgentJSON(t *testing.T) {
	keys, _ := crypto.GenerateEd25519KeyPair()
	agent, _ := identity.CreateAgent("test-agent", identity.AgentTypeGeneric, keys, identity.AgentMeta{})

	jsonData, err := agent.ToJSON()
	require.NoError(t, err)
	assert.Contains(t, string(jsonData), "test-agent")

	parsed, err := identity.FromJSON(jsonData)
	require.NoError(t, err)
	assert.Equal(t, agent.ID, parsed.ID)
	assert.Equal(t, agent.Name, parsed.Name)
}

func TestAgentPrettyPrint(t *testing.T) {
	keys, _ := crypto.GenerateEd25519KeyPair()
	agent, _ := identity.CreateAgent("test-agent", identity.AgentTypeGeneric, keys, identity.AgentMeta{})

	output := agent.PrettyPrint()
	assert.Contains(t, output, "test-agent")
	assert.Contains(t, output, agent.ID)
	assert.Contains(t, output, "Agent ID:")
}

func TestAgentValidateID(t *testing.T) {
	assert.True(t, identity.ValidateAgentID("aid:12345678"))
	assert.False(t, identity.ValidateAgentID("invalid"))
	assert.False(t, identity.ValidateAgentID("aid:123"))
}

func TestAgentParseID(t *testing.T) {
	info := identity.ParseAgentIDFull("aid:12345678")
	assert.NotNil(t, info)
	assert.Equal(t, "aid:", info.Prefix)
	assert.Equal(t, "12345678", info.Hash)
}

func TestAgentTypes(t *testing.T) {
	assert.Equal(t, identity.AgentType("generic"), identity.AgentTypeGeneric)
	assert.Equal(t, identity.AgentType("langchain"), identity.AgentTypeLangChain)
	assert.Equal(t, identity.AgentType("autogen"), identity.AgentTypeAutoGen)
	assert.Equal(t, identity.AgentType("crewai"), identity.AgentTypeCrewAI)
}

func TestAgentMeta(t *testing.T) {
	meta := identity.AgentMeta{
		Version:    "1.0.0",
		Framework:  "LangChain",
		Model:      "gpt-4",
		Owner:      "acme-corp",
		Tags:       []string{"production", "critical"},
		CustomData: map[string]string{"env": "prod"},
	}

	assert.Equal(t, "1.0.0", meta.Version)
	assert.Equal(t, "LangChain", meta.Framework)
	assert.Contains(t, meta.Tags, "production")
	assert.Equal(t, "prod", meta.CustomData["env"])
}

func TestIntentCreation(t *testing.T) {
	i := intent.CreateIntent(
		"Add user authentication",
		"Implement 2FA for all users",
		"AUTH-123",
		[]string{"Maintain backward compatibility"},
		[]string{"All tests pass"},
	)

	assert.NotNil(t, i)
	assert.Equal(t, "Add user authentication", i.Goal)
	assert.Equal(t, "Implement 2FA for all users", i.Description)
	assert.Equal(t, "AUTH-123", i.TicketID)
	assert.Equal(t, intent.IntentStatusOpen, i.Status)
	assert.Contains(t, i.ID, "int:")
	assert.Len(t, i.Constraints, 1)
	assert.Len(t, i.AcceptanceCriteria, 1)
}

func TestIntentLifecycle(t *testing.T) {
	i := intent.CreateIntent("test goal", "", "", nil, nil)

	assert.Equal(t, intent.IntentStatusOpen, i.Status)
	i.Progress()
	assert.Equal(t, intent.IntentStatusProgress, i.Status)
	i.Close(true)
	assert.Equal(t, intent.IntentStatusComplete, i.Status)
	assert.NotNil(t, i.ClosedAt)
}

func TestIntentCancel(t *testing.T) {
	i := intent.CreateIntent("test goal", "", "", nil, nil)
	i.Cancel()
	assert.Equal(t, intent.IntentStatusCanceled, i.Status)
	assert.NotNil(t, i.ClosedAt)
}

func TestIntentJSON(t *testing.T) {
	i := intent.CreateIntent("test goal", "description", "TICKET-123", []string{"constraint"}, []string{"criteria"})

	jsonData, err := i.ToJSON()
	require.NoError(t, err)
	assert.Contains(t, string(jsonData), "test goal")

	parsed, err := intent.FromJSON(jsonData)
	require.NoError(t, err)
	assert.Equal(t, i.ID, parsed.ID)
	assert.Equal(t, i.Goal, parsed.Goal)
}

func TestIntentPrettyPrint(t *testing.T) {
	i := intent.CreateIntent("test goal", "description", "TICKET-123", []string{"c1"}, []string{"c2"})
	output := i.PrettyPrint()
	assert.Contains(t, output, "test goal")
	assert.Contains(t, output, "TICKET-123")
}

func TestDBOpen(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("CGO required for go-sqlite3 on Windows")
	}
	db, err := storage.NewDB(":memory:")
	require.NoError(t, err)
	defer db.Close()
	assert.NotNil(t, db)
	assert.Equal(t, ":memory:", db.Path())
}

func TestDBMigrate(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("CGO required for go-sqlite3 on Windows")
	}
	db, err := storage.NewDB(":memory:")
	require.NoError(t, err)
	defer db.Close()

	err = db.Migrate()
	require.NoError(t, err)
}

func TestPolicyEngine(t *testing.T) {
	engine := policy.NewPolicyEngine()
	assert.NotNil(t, engine)
	assert.Greater(t, len(engine.ListPolicies()), 0)
}

func TestPolicyEvaluate(t *testing.T) {
	engine := policy.NewPolicyEngine()

	ctx := policy.ActionContext{
		Type:           "command",
		Target:         "rm -rf /tmp/test",
		Classification: "dangerous",
	}

	results := engine.Evaluate(ctx)
	assert.NotEmpty(t, results)

	var blocked bool
	for _, r := range results {
		if r.Matched && r.Action == policy.PolicyActionBlock {
			blocked = true
			break
		}
	}
	assert.True(t, blocked, "Should have blocked dangerous operation")
}

func TestPolicyAllow(t *testing.T) {
	engine := policy.NewPolicyEngine()

	ctx := policy.ActionContext{
		Type:           "command",
		Target:         "echo hello",
		Classification: "normal",
	}

	shouldAllow, results := engine.ShouldAllow(ctx)
	assert.True(t, shouldAllow)
	assert.NotNil(t, results)
}

func TestPolicyAdd(t *testing.T) {
	engine := policy.NewPolicyEngine()
	p := &policy.Policy{
		ID:        "custom-policy",
		Name:      "Custom Policy",
		Condition: policy.PolicyCondition{ActionType: []string{"custom"}},
		Action:    policy.PolicyActionWarn,
		Severity:  policy.SeverityInfo,
	}
	engine.AddPolicy(p)

	policies := engine.ListPolicies()
	var found bool
	for _, p := range policies {
		if p.ID == "custom-policy" {
			found = true
			break
		}
	}
	assert.True(t, found)
}

func TestPolicyRemove(t *testing.T) {
	engine := policy.NewPolicyEngine()
	engine.RemovePolicy("prevent-destructive")

	policies := engine.ListPolicies()
	for _, p := range policies {
		assert.NotEqual(t, "prevent-destructive", p.ID)
	}
}

func TestBackupManager(t *testing.T) {
	dir, err := os.MkdirTemp("", "attest-backup-test-*")
	require.NoError(t, err)
	defer os.RemoveAll(dir)

	bm, err := exec.NewBackupManager(dir)
	require.NoError(t, err)
	assert.NotNil(t, bm)

	testFile := dir + "/test.txt"
	err = os.WriteFile(testFile, []byte("test content"), 0644)
	require.NoError(t, err)

	backupPath, err := bm.CreateBackup(testFile, exec.BackupTypeFile)
	require.NoError(t, err)
	assert.NotEmpty(t, backupPath)
	assert.FileExists(t, backupPath)
}

func TestBackupRestore(t *testing.T) {
	dir, err := os.MkdirTemp("", "attest-backup-test-*")
	require.NoError(t, err)
	defer os.RemoveAll(dir)

	bm, err := exec.NewBackupManager(dir)
	require.NoError(t, err)

	testFile := dir + "/test.txt"
	err = os.WriteFile(testFile, []byte("original content"), 0644)
	require.NoError(t, err)

	backupPath, err := bm.CreateBackup(testFile, exec.BackupTypeFile)
	require.NoError(t, err)

	err = os.WriteFile(testFile, []byte("modified content"), 0644)
	require.NoError(t, err)

	err = bm.RestoreBackup(backupPath, testFile)
	require.NoError(t, err)

	content, _ := os.ReadFile(testFile)
	assert.Equal(t, "original content", string(content))
}

func TestExecutor(t *testing.T) {
	dir, err := os.MkdirTemp("", "attest-exec-test-*")
	require.NoError(t, err)
	defer os.RemoveAll(dir)

	executor, err := exec.NewExecutor(dir)
	require.NoError(t, err)
	assert.NotNil(t, executor)
}

func TestExecuteOptions(t *testing.T) {
	opts := exec.ExecuteOptions{
		Command:    "echo hello",
		WorkingDir: "/tmp",
		Reversible: true,
		BackupType: exec.BackupTypeFile,
		IntentID:   "int:123",
		AgentID:    "aid:456",
		DryRun:     false,
	}

	assert.Equal(t, "echo hello", opts.Command)
	assert.True(t, opts.Reversible)
	assert.Equal(t, exec.BackupTypeFile, opts.BackupType)
}

func TestReversibleStatus(t *testing.T) {
	assert.Equal(t, "pending", string(exec.StatusPending))
	assert.Equal(t, "executed", string(exec.StatusExecuted))
	assert.Equal(t, "rolled_back", string(exec.StatusRolledBack))
	assert.Equal(t, "failed", string(exec.StatusFailed))
}

func TestBackupType(t *testing.T) {
	assert.Equal(t, "file", string(exec.BackupTypeFile))
	assert.Equal(t, "directory", string(exec.BackupTypeDir))
	assert.Equal(t, "database", string(exec.BackupTypeDB))
	assert.Equal(t, "none", string(exec.BackupTypeNone))
}

func TestActionContext(t *testing.T) {
	ctx := policy.ActionContext{
		Type:           "command",
		Target:         "python script.py",
		Classification: "normal",
		AgentID:        "aid:123",
		IntentID:       "int:456",
		Environment:    "development",
		RiskLevel:      "low",
	}

	assert.Equal(t, "command", ctx.Type)
	assert.Equal(t, "python script.py", ctx.Target)
	assert.Equal(t, "development", ctx.Environment)
}

func TestPolicyResult(t *testing.T) {
	result := &policy.PolicyResult{
		PolicyID:   "test-policy",
		PolicyName: "Test Policy",
		Matched:    true,
		Action:     policy.PolicyActionBlock,
		Severity:   policy.SeverityCritical,
		Message:    "Test message",
	}

	assert.Equal(t, "test-policy", result.PolicyID)
	assert.True(t, result.Matched)
	assert.Equal(t, policy.PolicyActionBlock, result.Action)
}

func TestPolicyCondition(t *testing.T) {
	cond := policy.PolicyCondition{
		ActionType:     []string{"command", "database"},
		TargetMatch:    "dangerous",
		TargetRegex:    "rm.*",
		Classification: []string{"dangerous"},
		RiskLevel:      "high",
		Env:            "production",
	}

	assert.Len(t, cond.ActionType, 2)
	assert.Equal(t, "dangerous", cond.TargetMatch)
	assert.NotEmpty(t, cond.TargetRegex)
}

func TestIntentStatus(t *testing.T) {
	assert.Equal(t, "open", string(intent.IntentStatusOpen))
	assert.Equal(t, "in_progress", string(intent.IntentStatusProgress))
	assert.Equal(t, "completed", string(intent.IntentStatusComplete))
	assert.Equal(t, "failed", string(intent.IntentStatusFailed))
	assert.Equal(t, "canceled", string(intent.IntentStatusCanceled))
}

func TestIntentMeta(t *testing.T) {
	meta := intent.IntentMeta{
		Priority:   "high",
		Assignee:   "developer",
		Epic:       "AUTH-123",
		Labels:     []string{"security", "critical"},
		CustomData: map[string]string{"jira": "link"},
	}

	assert.Equal(t, "high", meta.Priority)
	assert.Equal(t, "developer", meta.Assignee)
	assert.Len(t, meta.Labels, 2)
}

func TestIntentGraph(t *testing.T) {
	i := intent.CreateIntent("test goal", "", "", nil, nil)
	graph := intent.IntentGraph{
		Root: i,
		Links: []intent.IntentLink{
			{
				IntentID:      i.ID,
				AttestationID: "att:123",
				ActionType:    "command",
				Timestamp:     "2024-01-01T00:00:00Z",
			},
		},
	}

	assert.NotNil(t, graph.Root)
	assert.Len(t, graph.Links, 1)
	assert.Equal(t, i.ID, graph.Links[0].IntentID)
}

func TestIntentInfo(t *testing.T) {
	i := intent.CreateIntent("test goal", "", "", nil, nil)
	info := i.ToDisplayInfo(5)

	assert.Equal(t, i.ID, info.ID)
	assert.Equal(t, i.Goal, info.Goal)
	assert.Equal(t, i.TicketID, info.TicketID)
	assert.Equal(t, i.Status, info.Status)
	assert.Equal(t, 5, info.Actions)
}

func TestAgentInfo(t *testing.T) {
	keys, _ := crypto.GenerateEd25519KeyPair()
	agent, _ := identity.CreateAgent("test", identity.AgentTypeGeneric, keys, identity.AgentMeta{})

	info := agent.ToDisplayInfo()
	assert.Equal(t, agent.ID, info.ID)
	assert.Equal(t, agent.Name, info.Name)
	assert.Equal(t, string(agent.Type), info.Type)
	assert.Equal(t, agent.CreatedAt.Unix(), info.CreatedAt.Unix())
}

func TestAttestationActionTypes(t *testing.T) {
	types := []attestation.ActionType{
		attestation.ActionTypeCommand,
		attestation.ActionTypeFileEdit,
		attestation.ActionTypeAPICall,
		attestation.ActionTypeDatabase,
		attestation.ActionTypeGit,
		attestation.ActionTypeCustom,
	}

	assert.Len(t, types, 6)
	assert.Equal(t, "command", string(attestation.ActionTypeCommand))
	assert.Equal(t, "git", string(attestation.ActionTypeGit))
}

package integrations

import (
	"fmt"
	"os"
	"path/filepath"
)

func CreateConfig(framework string) (string, error) {
	configContent := generateConfig(framework)
	configPath := "attest.yaml"

	err := os.WriteFile(configPath, []byte(configContent), 0644)
	if err != nil {
		return "", err
	}

	return configPath, nil
}

func generateConfig(framework string) string {
	return fmt.Sprintf(`# Attest Configuration
version: "1.0"

# Framework settings
framework: %s

# Validation rules
validation:
  prompt_injection: true
  output_validation: true
  tool_monitoring: true
  cost_tracking: true

# Test settings
testing:
  run_on_commit: true
  min_coverage: 80
  scenarios:
    - name: basic_functionality
      description: "Basic agent functionality test"
    - name: edge_cases
      description: "Edge case handling"
    - name: safety_checks
      description: "Safety and security validation"

# Monitoring
monitoring:
  logging: true
  metrics: true
  alerting: true

# Integration settings
integration:
  git_hooks: true
  ci_integration: true
  ide_support: true
`, framework)
}

func InstallFrameworkHooks(framework string) (string, error) {
	var templateContent string
	var outputFile string

	switch framework {
	case "langchain":
		outputFile = "attest_callback.py"
		templateContent = GetLangChainTemplate()
	case "autogen":
		outputFile = "attest_autogen_setup.py"
		templateContent = GetAutoGenTemplate()
	case "crewai":
		outputFile = "attest_crew_setup.py"
		templateContent = GetCrewAITemplate()
	default:
		return "", fmt.Errorf("unsupported framework: %s", framework)
	}

	err := os.MkdirAll("attest", 0755)
	if err != nil {
		return "", err
	}

	outputPath := filepath.Join("attest", outputFile)
	err = os.WriteFile(outputPath, []byte(templateContent), 0644)
	if err != nil {
		return "", err
	}

	return outputPath, nil
}

func SetupGitHooks() error {
	if _, err := os.Stat(".git"); os.IsNotExist(err) {
		return fmt.Errorf("not a git repository")
	}

	hooksDir := filepath.Join(".git", "hooks")

	preCommitHook := `#!/bin/bash
# Attest Pre-Commit Hook
echo "Running Attest validation..."

if ! command -v attest &> /dev/null; then
    echo "Attest not found. Install with: pip install attest"
    exit 0
fi

attest validate
exit $?
`

	preCommitPath := filepath.Join(hooksDir, "pre-commit")
	err := os.WriteFile(preCommitPath, []byte(preCommitHook), 0755)
	if err != nil {
		return err
	}

	postCommitHook := `#!/bin/bash
# Attest Post-Commit Hook
echo "Sending metrics to Attest dashboard..."

if command -v attest &> /dev/null; then
    attest log-commit --silent &
fi
`

	postCommitPath := filepath.Join(hooksDir, "post-commit")
	return os.WriteFile(postCommitPath, []byte(postCommitHook), 0755)
}

func SetupCITemplates(framework string) (string, error) {
	workflowDir := filepath.Join(".github", "workflows")
	err := os.MkdirAll(workflowDir, 0755)
	if err != nil {
		return "", err
	}

	workflowContent := generateWorkflow(framework)
	workflowPath := filepath.Join(workflowDir, "attest.yml")

	err = os.WriteFile(workflowPath, []byte(workflowContent), 0644)
	if err != nil {
		return "", err
	}

	return workflowPath, nil
}

func generateWorkflow(framework string) string {
	var frameworkInstall string
	switch framework {
	case "langchain":
		frameworkInstall = `
      - name: Install LangChain dependencies
        run: pip install langchain langchain-openai`
	case "autogen":
		frameworkInstall = `
      - name: Install AutoGen dependencies
        run: pip install pyautogen`
	case "crewai":
		frameworkInstall = `
      - name: Install CrewAI dependencies
        run: pip install crewai`
	}

	return fmt.Sprintf(`name: Attest Validation

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  validate:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'

      - name: Install dependencies
        run: |
          python -m pip install --upgrade pip
          pip install -r requirements.txt || true%s
          pip install attest

      - name: Run Attest validation
        run: attest validate --ci
        env:
          ATTEST_API_KEY: ${{ secrets.ATTEST_API_KEY }}

      - name: Upload results
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: attest-results
          path: attest-results/
`, frameworkInstall)
}

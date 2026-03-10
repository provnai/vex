package setup

import (
	"bufio"
	"os"
	"path/filepath"
	"strings"
)

func DetectFramework() string {
	// Check for Python frameworks
	if checkRequirementsTxt() {
		return detectFromRequirements()
	}

	// Check for package.json (Node.js projects)
	if checkPackageJSON() {
		return detectFromPackageJSON()
	}

	// Check for specific files
	if fileExists("crewai_config.yaml") || fileExists("crew.yaml") {
		return "crewai"
	}

	if fileExists("autogen_config.json") {
		return "autogen"
	}

	if fileExists("langchain.yaml") {
		return "langchain"
	}

	// Check source files
	if hasPythonImport("langchain") {
		return "langchain"
	}
	if hasPythonImport("autogen") || hasPythonImport("pyautogen") {
		return "autogen"
	}
	if hasPythonImport("crewai") {
		return "crewai"
	}

	return ""
}

func checkRequirementsTxt() bool {
	return fileExists("requirements.txt") || fileExists("requirements-dev.txt")
}

func checkPackageJSON() bool {
	return fileExists("package.json")
}

func detectFromRequirements() string {
	files := []string{"requirements.txt", "requirements-dev.txt"}

	for _, file := range files {
		framework := parseRequirementsFile(file)
		if framework != "" {
			return framework
		}
	}

	return ""
}

func parseRequirementsFile(filename string) string {
	file, err := os.Open(filename)
	if err != nil {
		return ""
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.ToLower(scanner.Text())
		line = strings.Split(line, "#")[0] // Remove comments
		line = strings.Split(line, ";")[0] // Remove environment markers
		line = strings.TrimSpace(line)

		if strings.Contains(line, "langchain") {
			return "langchain"
		}
		if strings.Contains(line, "autogen") || strings.Contains(line, "pyautogen") {
			return "autogen"
		}
		if strings.Contains(line, "crewai") {
			return "crewai"
		}
	}

	return ""
}

func detectFromPackageJSON() string {
	file, err := os.Open("package.json")
	if err != nil {
		return ""
	}
	defer file.Close()

	// Simple string search for framework indicators
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.ToLower(scanner.Text())
		if strings.Contains(line, "langchain") {
			return "langchain"
		}
	}

	return ""
}

func hasPythonImport(module string) bool {
	pythonFiles, _ := filepath.Glob("*.py")
	for _, file := range pythonFiles {
		content, err := os.ReadFile(file)
		if err != nil {
			continue
		}
		if strings.Contains(string(content), "import "+module) ||
			strings.Contains(string(content), "from "+module) {
			return true
		}
	}
	return false
}

func fileExists(filename string) bool {
	_, err := os.Stat(filename)
	return !os.IsNotExist(err)
}

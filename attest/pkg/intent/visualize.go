package intent

import (
	"fmt"
	"strings"
)

// TreeNode represents a node in the intent tree
type TreeNode struct {
	Intent   *Intent
	Actions  []*ActionNode
	Children []*TreeNode
	Depth    int
}

// ActionNode represents an action linked to an intent
type ActionNode struct {
	AttestationID string
	ActionType    string
	Target        string
	Timestamp     string
	Status        string
}

// VisualizeIntentTree creates an ASCII tree visualization
func VisualizeIntentTree(root *Intent, actions []*ActionNode, children []*TreeNode) string {
	if root == nil {
		return "(empty)"
	}

	var sb strings.Builder
	sb.WriteString(renderNode(root, 0, true, true))

	if len(actions) > 0 {
		sb.WriteString("\n" + strings.Repeat(" ", 4) + "└─ Actions:\n")
		for i, action := range actions {
			isLast := i == len(actions)-1
			prefix := strings.Repeat(" ", 6)
			if isLast {
				sb.WriteString(prefix + "└─ ")
			} else {
				sb.WriteString(prefix + "├─ ")
			}
			sb.WriteString(fmt.Sprintf("[%s] %s → %s (%s)\n",
				action.ActionType,
				truncate(action.AttestationID, 12),
				truncate(action.Target, 30),
				action.Timestamp[:10]))
		}
	}

	if len(children) > 0 {
		sb.WriteString("\n" + strings.Repeat(" ", 4) + "└─ Sub-intents:\n")
		for i, child := range children {
			isLast := i == len(children)-1
			sb.WriteString(renderIntentChild(child, 6, isLast))
		}
	}

	return sb.String()
}

func renderNode(i *Intent, depth int, isRoot bool, isLast bool) string {
	var sb strings.Builder
	indent := strings.Repeat("  ", depth)

	var prefix string
	if isRoot {
		prefix = "└─ "
	} else if isLast {
		prefix = "└─ "
	} else {
		prefix = "├─ "
	}

	statusIcon := statusIcon(i.Status)
	sb.WriteString(fmt.Sprintf("%s%s %s %s\n",
		indent+prefix,
		statusIcon,
		truncate(i.ID, 12),
		truncate(i.Goal, 40)))

	if i.TicketID != "" {
		sb.WriteString(fmt.Sprintf("%s   └─ Ticket: %s\n", indent, i.TicketID))
	}

	return sb.String()
}

func renderIntentChild(node *TreeNode, depth int, isLast bool) string {
	var sb strings.Builder
	indent := strings.Repeat("  ", depth)

	prefix := "└─ "
	if !isLast {
		prefix = "├─ "
	}

	statusIcon := statusIcon(node.Intent.Status)
	sb.WriteString(fmt.Sprintf("%s%s %s %s\n",
		indent+prefix,
		statusIcon,
		truncate(node.Intent.ID, 12),
		truncate(node.Intent.Goal, 40)))

	// Render child actions
	for i, action := range node.Actions {
		childIndent := strings.Repeat("  ", depth+2)
		actionPrefix := "└─ "
		if i < len(node.Actions)-1 {
			actionPrefix = "├─ "
		}
		sb.WriteString(fmt.Sprintf("%s%s [%s] %s\n",
			childIndent+actionPrefix,
			action.ActionType,
			truncate(action.AttestationID, 12),
			truncate(action.Target, 25)))
	}

	// Render sub-children
	for i, child := range node.Children {
		sb.WriteString(renderIntentChild(child, depth+2, i == len(node.Children)-1))
	}

	return sb.String()
}

func statusIcon(status IntentStatus) string {
	switch status {
	case IntentStatusOpen:
		return "○"
	case IntentStatusProgress:
		return "◐"
	case IntentStatusComplete:
		return "✓"
	case IntentStatusFailed:
		return "✗"
	case IntentStatusCanceled:
		return "⊘"
	default:
		return "?"
	}
}

func truncate(s string, maxLen int) string {
	if len(s) <= maxLen {
		return s
	}
	return s[:maxLen-2] + ".."
}

// ExportDOT exports the intent graph in DOT format for Graphviz
func ExportDOT(root *Intent, actions []*ActionNode, children []*TreeNode) string {
	var sb strings.Builder
	sb.WriteString("digraph IntentGraph {\n")
	sb.WriteString("  rankdir=TB;\n")
	sb.WriteString("  node [shape=box, style=rounded];\n\n")

	// Root intent
	sb.WriteString(fmt.Sprintf("  %s [label=%q, fillcolor=lightblue, style=filled];\n",
		root.ID, fmt.Sprintf("%s\\n%s", truncate(root.ID, 12), truncate(root.Goal, 30))))

	// Actions
	for _, action := range actions {
		color := actionColor(action.ActionType)
		sb.WriteString(fmt.Sprintf("  %s [label=%q, fillcolor=%s, style=filled];\n",
			action.AttestationID,
			fmt.Sprintf("%s\\n%s", action.ActionType, truncate(action.Target, 20)),
			color))
		sb.WriteString(fmt.Sprintf("  %s -> %s;\n", root.ID, action.AttestationID))
	}

	// Children
	for _, child := range children {
		sb.WriteString(exportDOTNode(child, root.ID))
	}

	sb.WriteString("}\n")
	return sb.String()
}

func exportDOTNode(node *TreeNode, parentID string) string {
	var sb strings.Builder

	color := statusColor(node.Intent.Status)
	sb.WriteString(fmt.Sprintf("  %s [label=%q, fillcolor=%s, style=filled];\n",
		node.Intent.ID,
		fmt.Sprintf("%s\\n%s", truncate(node.Intent.ID, 12), truncate(node.Intent.Goal, 25)),
		color))
	sb.WriteString(fmt.Sprintf("  %s -> %s;\n", parentID, node.Intent.ID))

	for _, action := range node.Actions {
		actionColor := actionColor(action.ActionType)
		sb.WriteString(fmt.Sprintf("  %s [label=%q, fillcolor=%s, style=filled];\n",
			action.AttestationID,
			fmt.Sprintf("%s\\n%s", action.ActionType, truncate(action.Target, 20)),
			actionColor))
		sb.WriteString(fmt.Sprintf("  %s -> %s;\n", node.Intent.ID, action.AttestationID))
	}

	for _, child := range node.Children {
		sb.WriteString(exportDOTNode(child, node.Intent.ID))
	}

	return sb.String()
}

func actionColor(actionType string) string {
	switch actionType {
	case "command":
		return "lightyellow"
	case "database":
		return "lightcoral"
	case "api_call":
		return "lightgreen"
	case "file_edit":
		return "lightblue"
	case "git":
		return "lavender"
	default:
		return "lightgray"
	}
}

func statusColor(status IntentStatus) string {
	switch status {
	case IntentStatusOpen:
		return "lightyellow"
	case IntentStatusProgress:
		return "lightskyblue"
	case IntentStatusComplete:
		return "lightgreen"
	case IntentStatusFailed:
		return "lightcoral"
	case IntentStatusCanceled:
		return "lightgray"
	default:
		return "lightgray"
	}
}

// GenerateASCIIProgressBar creates a progress bar
func GenerateASCIIProgressBar(current, total int, width int) string {
	if total == 0 {
		return strings.Repeat("░", width)
	}

	percent := float64(current) / float64(total)
	filled := int(float64(width) * percent)

	filledStr := strings.Repeat("█", filled)
	emptyStr := strings.Repeat("░", width-filled)

	return filledStr + emptyStr
}

// FormatIntentSummary creates a compact summary
func FormatIntentSummary(i *Intent, actionCount int) string {
	progress := GenerateASCIIProgressBar(0, 1, 10)
	if i.Status == IntentStatusComplete {
		progress = GenerateASCIIProgressBar(1, 1, 10)
	} else if i.Status == IntentStatusProgress {
		progress = GenerateASCIIProgressBar(1, 2, 10)
	}

	return fmt.Sprintf("[%s] %s %s (%d actions)",
		progress,
		truncate(i.ID, 8),
		truncate(i.Goal, 25),
		actionCount)
}

package cost

import (
	"testing"
)

func TestCalculateCost(t *testing.T) {
	tests := []struct {
		name         string
		modelID      string
		inputTokens  int64
		outputTokens int64
		expectError  bool
	}{
		{
			name:         "GPT-4 standard call",
			modelID:      "gpt-4",
			inputTokens:  1000,
			outputTokens: 500,
			expectError:  false,
		},
		{
			name:         "GPT-3.5-turbo call",
			modelID:      "gpt-3.5-turbo",
			inputTokens:  5000,
			outputTokens: 2000,
			expectError:  false,
		},
		{
			name:         "Claude-3-opus call",
			modelID:      "claude-3-opus",
			inputTokens:  2000,
			outputTokens: 1000,
			expectError:  false,
		},
		{
			name:         "Unknown model",
			modelID:      "unknown-model-xyz",
			inputTokens:  1000,
			outputTokens: 500,
			expectError:  true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cost, err := CalculateCost(tt.modelID, tt.inputTokens, tt.outputTokens)

			if tt.expectError {
				if err == nil {
					t.Error("expected error for unknown model, got nil")
				}
				return
			}

			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}

			if cost < 0 {
				t.Errorf("cost should be non-negative, got %v", cost)
			}
		})
	}
}

func TestFormatCurrency(t *testing.T) {
	tests := []struct {
		amount   float64
		expected string
	}{
		{0, "$0.00"},
		{0.5, "$0.50"},
		{1.0, "$1.00"},
		{10.99, "$10.99"},
		{100.0, "$100.00"},
		{0.001, "$0.00"},
		{0.009, "$0.01"},
	}

	for _, tt := range tests {
		t.Run(tt.expected, func(t *testing.T) {
			result := FormatCurrency(tt.amount)
			if result != tt.expected {
				t.Errorf("FormatCurrency(%v) = %v, want %v", tt.amount, result, tt.expected)
			}
		})
	}
}

func TestBudgetExceededError(t *testing.T) {
	err := &BudgetExceededError{
		Period:     PeriodDaily,
		Spent:      15.50,
		Limit:      10.00,
		Percentage: 1.55,
	}

	expected := "Budget exceeded: $15.50/$10.00 (155.0%) for daily period"
	if err.Error() != expected {
		t.Errorf("Error() = %v, want %v", err.Error(), expected)
	}
}

func TestDefaultBudgetConfig(t *testing.T) {
	config := DefaultBudgetConfig()

	if config.DailyLimit != 10.0 {
		t.Errorf("DailyLimit = %v, want 10.0", config.DailyLimit)
	}
	if config.WeeklyLimit != 50.0 {
		t.Errorf("WeeklyLimit = %v, want 50.0", config.WeeklyLimit)
	}
	if config.MonthlyLimit != 200.0 {
		t.Errorf("MonthlyLimit = %v, want 200.0", config.MonthlyLimit)
	}
	if !config.HardStop {
		t.Error("HardStop should be true by default")
	}
	if config.WarnThreshold != 0.8 {
		t.Errorf("WarnThreshold = %v, want 0.8", config.WarnThreshold)
	}
}

func TestBudgetPeriodConstants(t *testing.T) {
	if PeriodDaily != "daily" {
		t.Errorf("PeriodDaily = %v, want 'daily'", PeriodDaily)
	}
	if PeriodWeekly != "weekly" {
		t.Errorf("PeriodWeekly = %v, want 'weekly'", PeriodWeekly)
	}
	if PeriodMonthly != "monthly" {
		t.Errorf("PeriodMonthly = %v, want 'monthly'", PeriodMonthly)
	}
}

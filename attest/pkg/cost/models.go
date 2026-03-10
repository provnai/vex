// pkg/cost/models.go - API pricing data for different models

package cost

import (
	"fmt"
	"time"
)

// ModelPricing defines cost per 1K tokens for input and output
type ModelPricing struct {
	ModelID     string
	Provider    string
	InputPrice  float64 // Cost per 1K input tokens in USD
	OutputPrice float64 // Cost per 1K output tokens in USD
	Currency    string
}

// BudgetPeriod defines the time period for budget tracking
type BudgetPeriod string

const (
	PeriodDaily   BudgetPeriod = "daily"
	PeriodWeekly  BudgetPeriod = "weekly"
	PeriodMonthly BudgetPeriod = "monthly"
)

// BudgetConfig holds user-defined budget limits
type BudgetConfig struct {
	ID            int64
	DailyLimit    float64
	WeeklyLimit   float64
	MonthlyLimit  float64
	HardStop      bool    // If true, execution stops when budget exceeded
	WarnThreshold float64 // Percentage (0.0-1.0) to trigger warning
	UpdatedAt     time.Time
}

// DefaultBudgetConfig returns default budget configuration
func DefaultBudgetConfig() *BudgetConfig {
	return &BudgetConfig{
		DailyLimit:    10.0,  // $10/day default
		WeeklyLimit:   50.0,  // $50/week default
		MonthlyLimit:  200.0, // $200/month default
		HardStop:      true,
		WarnThreshold: 0.8, // Warn at 80%
	}
}

// CostEntry represents a single API call cost record
type CostEntry struct {
	ID           int64
	Date         time.Time
	Model        string
	Provider     string
	InputTokens  int64
	OutputTokens int64
	InputCost    float64
	OutputCost   float64
	TotalCost    float64
	Cumulative   float64
	RunID        string
	CreatedAt    time.Time
}

// SpendingReport summarizes costs over a period
type SpendingReport struct {
	Period      BudgetPeriod
	StartDate   time.Time
	EndDate     time.Time
	TotalSpent  float64
	BudgetLimit float64
	Percentage  float64
	ByModel     map[string]float64
	ByProvider  map[string]float64
	Entries     []CostEntry
}

// KnownModelPricing - comprehensive pricing database
var KnownModelPricing = map[string]ModelPricing{
	// OpenAI Models
	"gpt-4": {
		ModelID:     "gpt-4",
		Provider:    "openai",
		InputPrice:  0.03,
		OutputPrice: 0.06,
		Currency:    "USD",
	},
	"gpt-4-turbo": {
		ModelID:     "gpt-4-turbo",
		Provider:    "openai",
		InputPrice:  0.01,
		OutputPrice: 0.03,
		Currency:    "USD",
	},
	"gpt-4o": {
		ModelID:     "gpt-4o",
		Provider:    "openai",
		InputPrice:  0.005,
		OutputPrice: 0.015,
		Currency:    "USD",
	},
	"gpt-4o-mini": {
		ModelID:     "gpt-4o-mini",
		Provider:    "openai",
		InputPrice:  0.00015,
		OutputPrice: 0.0006,
		Currency:    "USD",
	},
	"gpt-3.5-turbo": {
		ModelID:     "gpt-3.5-turbo",
		Provider:    "openai",
		InputPrice:  0.0005,
		OutputPrice: 0.0015,
		Currency:    "USD",
	},

	// Anthropic Models
	"claude-3-opus": {
		ModelID:     "claude-3-opus",
		Provider:    "anthropic",
		InputPrice:  0.015,
		OutputPrice: 0.075,
		Currency:    "USD",
	},
	"claude-3-sonnet": {
		ModelID:     "claude-3-sonnet",
		Provider:    "anthropic",
		InputPrice:  0.003,
		OutputPrice: 0.015,
		Currency:    "USD",
	},
	"claude-3-haiku": {
		ModelID:     "claude-3-haiku",
		Provider:    "anthropic",
		InputPrice:  0.00025,
		OutputPrice: 0.00125,
		Currency:    "USD",
	},
	"claude-3-5-sonnet": {
		ModelID:     "claude-3-5-sonnet",
		Provider:    "anthropic",
		InputPrice:  0.003,
		OutputPrice: 0.015,
		Currency:    "USD",
	},

	// Google Models
	"gemini-pro": {
		ModelID:     "gemini-pro",
		Provider:    "google",
		InputPrice:  0.0005,
		OutputPrice: 0.0015,
		Currency:    "USD",
	},
	"gemini-ultra": {
		ModelID:     "gemini-ultra",
		Provider:    "google",
		InputPrice:  0.0035,
		OutputPrice: 0.0105,
		Currency:    "USD",
	},
}

// GetPricing returns pricing for a model, with fallback
func GetPricing(modelID string) (ModelPricing, error) {
	if pricing, ok := KnownModelPricing[modelID]; ok {
		return pricing, nil
	}

	// Try to match by prefix
	for id, pricing := range KnownModelPricing {
		if len(modelID) >= len(id) && modelID[:len(id)] == id {
			return pricing, nil
		}
	}

	return ModelPricing{}, fmt.Errorf("unknown model: %s", modelID)
}

// CalculateCost computes total cost for a model call
func CalculateCost(modelID string, inputTokens, outputTokens int64) (float64, error) {
	pricing, err := GetPricing(modelID)
	if err != nil {
		return 0, err
	}

	inputCost := (float64(inputTokens) / 1000.0) * pricing.InputPrice
	outputCost := (float64(outputTokens) / 1000.0) * pricing.OutputPrice

	return inputCost + outputCost, nil
}

// FormatCurrency formats a cost value as currency string
func FormatCurrency(amount float64) string {
	return fmt.Sprintf("$%.2f", amount)
}

// BudgetExceededError is returned when budget limit is reached
type BudgetExceededError struct {
	Period     BudgetPeriod
	Spent      float64
	Limit      float64
	Percentage float64
}

func (e *BudgetExceededError) Error() string {
	return fmt.Sprintf("Budget exceeded: %s/%s (%.1f%%) for %s period",
		FormatCurrency(e.Spent), FormatCurrency(e.Limit), e.Percentage*100, e.Period)
}

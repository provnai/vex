package cost

import (
	"testing"
)

func TestCalculateCostFromPricing(t *testing.T) {
	tests := []struct {
		name         string
		modelID      string
		inputTokens  int64
		outputTokens int64
		expectedCost float64
	}{
		{
			name:         "GPT-4 small request",
			modelID:      "gpt-4",
			inputTokens:  1000,
			outputTokens: 1000,
			expectedCost: 0.09, // $0.03 + $0.06
		},
		{
			name:         "GPT-4 large request",
			modelID:      "gpt-4",
			inputTokens:  100000,
			outputTokens: 50000,
			expectedCost: 6.0, // 100 * 0.03 + 50 * 0.06
		},
		{
			name:         "GPT-3.5-turbo",
			modelID:      "gpt-3.5-turbo",
			inputTokens:  10000,
			outputTokens: 5000,
			expectedCost: 0.01, // 10 * 0.0005 + 5 * 0.0015 = 0.005 + 0.0075 = 0.0125 -> 0.01
		},
		{
			name:         "Claude-3-opus",
			modelID:      "claude-3-opus",
			inputTokens:  1000,
			outputTokens: 1000,
			expectedCost: 0.09, // 0.015 + 0.075
		},
		{
			name:         "Claude-3-haiku",
			modelID:      "claude-3-haiku",
			inputTokens:  10000,
			outputTokens: 5000,
			expectedCost: 0.01, // 10 * 0.00025 + 5 * 0.00125 = 0.0025 + 0.00625 = 0.00875 -> 0.01
		},
		{
			name:         "GPT-4o-mini",
			modelID:      "gpt-4o-mini",
			inputTokens:  100000,
			outputTokens: 10000,
			expectedCost: 0.02, // 100 * 0.00015 + 10 * 0.0006 = 0.015 + 0.006 = 0.021 -> 0.02
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			pricing, err := GetPricingData(tt.modelID)
			if err != nil {
				t.Fatalf("failed to get pricing for %s: %v", tt.modelID, err)
			}

			cost := CalculateCostFromPricing(pricing, tt.inputTokens, tt.outputTokens)

			if cost != tt.expectedCost {
				t.Errorf("expected cost %v, got %v", tt.expectedCost, cost)
			}
		})
	}
}

func TestGetPricingData(t *testing.T) {
	tests := []struct {
		modelID      string
		wantErr      bool
		wantProvider string
	}{
		{"gpt-4", false, "openai"},
		{"gpt-3.5-turbo", false, "openai"},
		{"claude-3-opus", false, "anthropic"},
		{"claude-3-haiku", false, "anthropic"},
		{"gemini-pro", false, "google"},
		{"unknown-model", true, ""},
	}

	for _, tt := range tests {
		t.Run(tt.modelID, func(t *testing.T) {
			pricing, err := GetPricingData(tt.modelID)
			if tt.wantErr {
				if err == nil {
					t.Error("expected error for unknown model, got nil")
				}
				return
			}
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if pricing.Provider != tt.wantProvider {
				t.Errorf("provider = %v, want %v", pricing.Provider, tt.wantProvider)
			}
		})
	}
}

func TestRoundToCents(t *testing.T) {
	tests := []struct {
		input    float64
		expected float64
	}{
		{0.125, 0.13},
		{0.124, 0.12},
		{0.001, 0.0},
		{0.009, 0.01},
		{1.0, 1.0},
		{0.005, 0.01},
		{0.004, 0.0},
	}

	for _, tt := range tests {
		result := roundToCents(tt.input)
		if result != tt.expected {
			t.Errorf("roundToCents(%v) = %v, want %v", tt.input, result, tt.expected)
		}
	}
}

func TestGetPricingForProvider(t *testing.T) {
	tests := []struct {
		provider  string
		wantCount int
	}{
		{"openai", len(OpenAIPricing)},
		{"anthropic", len(AnthropicPricing)},
		{"google", len(GooglePricing)},
		{"unknown", 0},
	}

	for _, tt := range tests {
		t.Run(tt.provider, func(t *testing.T) {
			result := GetPricingForProvider(tt.provider)
			if tt.wantCount == 0 && result != nil {
				t.Errorf("expected nil for unknown provider, got %v", result)
			}
			if tt.wantCount > 0 && len(result) != tt.wantCount {
				t.Errorf("len(%s) = %v, want %v", tt.provider, len(result), tt.wantCount)
			}
		})
	}
}

func TestAddOrUpdatePricing(t *testing.T) {
	initialLen := len(AllPricing)

	newPricing := PricingData{
		Provider:       "test",
		ModelID:        "test-model",
		InputPriceUSD:  0.001,
		OutputPriceUSD: 0.002,
		Unit:           "1K tokens",
	}

	AddOrUpdatePricing("test-model", newPricing)
	defer func() {
		pricingMu.Lock()
		delete(AllPricing, "test-model")
		pricingMu.Unlock()
	}()

	if len(AllPricing) != initialLen+1 {
		t.Errorf("expected length %d, got %d", initialLen+1, len(AllPricing))
	}

	retrieved, err := GetPricingData("test-model")
	if err != nil {
		t.Fatalf("failed to get newly added pricing: %v", err)
	}

	if retrieved.Provider != "test" {
		t.Errorf("provider = %v, want %v", retrieved.Provider, "test")
	}
	if retrieved.InputPriceUSD != 0.001 {
		t.Errorf("input price = %v, want %v", retrieved.InputPriceUSD, 0.001)
	}
}

func TestAllPricingPopulated(t *testing.T) {
	totalExpected := len(OpenAIPricing) + len(AnthropicPricing) + len(GooglePricing)
	
	pricingMu.RLock()
	actualLen := len(AllPricing)
	pricingMu.RUnlock()
	
	if actualLen != totalExpected {
		t.Errorf("AllPricing has %d entries, expected %d", actualLen, totalExpected)
	}
}

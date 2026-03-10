// pkg/cost/pricing.go - API pricing data for different models and providers

package cost

import "fmt"

type PricingData struct {
	Provider       string
	ModelID        string
	InputPriceUSD  float64
	OutputPriceUSD float64
	Unit           string
}

var OpenAIPricing = map[string]PricingData{
	"gpt-4": {
		Provider:       "openai",
		ModelID:        "gpt-4",
		InputPriceUSD:  0.03,
		OutputPriceUSD: 0.06,
		Unit:           "1K tokens",
	},
	"gpt-4-turbo": {
		Provider:       "openai",
		ModelID:        "gpt-4-turbo",
		InputPriceUSD:  0.01,
		OutputPriceUSD: 0.03,
		Unit:           "1K tokens",
	},
	"gpt-4o": {
		Provider:       "openai",
		ModelID:        "gpt-4o",
		InputPriceUSD:  0.005,
		OutputPriceUSD: 0.015,
		Unit:           "1K tokens",
	},
	"gpt-4o-mini": {
		Provider:       "openai",
		ModelID:        "gpt-4o-mini",
		InputPriceUSD:  0.00015,
		OutputPriceUSD: 0.0006,
		Unit:           "1K tokens",
	},
	"gpt-3.5-turbo": {
		Provider:       "openai",
		ModelID:        "gpt-3.5-turbo",
		InputPriceUSD:  0.0005,
		OutputPriceUSD: 0.0015,
		Unit:           "1K tokens",
	},
	"gpt-3.5-turbo-16k": {
		Provider:       "openai",
		ModelID:        "gpt-3.5-turbo-16k",
		InputPriceUSD:  0.003,
		OutputPriceUSD: 0.004,
		Unit:           "1K tokens",
	},
}

var AnthropicPricing = map[string]PricingData{
	"claude-3-opus": {
		Provider:       "anthropic",
		ModelID:        "claude-3-opus",
		InputPriceUSD:  0.015,
		OutputPriceUSD: 0.075,
		Unit:           "1K tokens",
	},
	"claude-3-sonnet": {
		Provider:       "anthropic",
		ModelID:        "claude-3-sonnet",
		InputPriceUSD:  0.003,
		OutputPriceUSD: 0.015,
		Unit:           "1K tokens",
	},
	"claude-3-haiku": {
		Provider:       "anthropic",
		ModelID:        "claude-3-haiku",
		InputPriceUSD:  0.00025,
		OutputPriceUSD: 0.00125,
		Unit:           "1K tokens",
	},
	"claude-3-5-sonnet": {
		Provider:       "anthropic",
		ModelID:        "claude-3-5-sonnet",
		InputPriceUSD:  0.003,
		OutputPriceUSD: 0.015,
		Unit:           "1K tokens",
	},
	"claude-3-5-haiku": {
		Provider:       "anthropic",
		ModelID:        "claude-3-5-haiku",
		InputPriceUSD:  0.00025,
		OutputPriceUSD: 0.00125,
		Unit:           "1K tokens",
	},
}

var GooglePricing = map[string]PricingData{
	"gemini-pro": {
		Provider:       "google",
		ModelID:        "gemini-pro",
		InputPriceUSD:  0.0005,
		OutputPriceUSD: 0.0015,
		Unit:           "1K tokens",
	},
	"gemini-ultra": {
		Provider:       "google",
		ModelID:        "gemini-ultra",
		InputPriceUSD:  0.0035,
		OutputPriceUSD: 0.0105,
		Unit:           "1K tokens",
	},
	"gemini-1.5-pro": {
		Provider:       "google",
		ModelID:        "gemini-1.5-pro",
		InputPriceUSD:  0.00035,
		OutputPriceUSD: 0.0007,
		Unit:           "1K tokens",
	},
	"gemini-1.5-flash": {
		Provider:       "google",
		ModelID:        "gemini-1.5-flash",
		InputPriceUSD:  0.000075,
		OutputPriceUSD: 0.0003,
		Unit:           "1K tokens",
	},
}

var AllPricing = map[string]PricingData{}

func init() {
	for k, v := range OpenAIPricing {
		AllPricing[k] = v
	}
	for k, v := range AnthropicPricing {
		AllPricing[k] = v
	}
	for k, v := range GooglePricing {
		AllPricing[k] = v
	}
}

func GetPricingData(modelID string) (PricingData, error) {
	if pricing, ok := AllPricing[modelID]; ok {
		return pricing, nil
	}
	return PricingData{}, fmt.Errorf("unknown model: %s", modelID)
}

func GetPricingForProvider(provider string) map[string]PricingData {
	switch provider {
	case "openai":
		return OpenAIPricing
	case "anthropic":
		return AnthropicPricing
	case "google":
		return GooglePricing
	default:
		return nil
	}
}

func CalculateCostFromPricing(pricing PricingData, inputTokens, outputTokens int64) float64 {
	inputCost := (float64(inputTokens) / 1000.0) * pricing.InputPriceUSD
	outputCost := (float64(outputTokens) / 1000.0) * pricing.OutputPriceUSD
	return roundToCents(inputCost + outputCost)
}

func roundToCents(amount float64) float64 {
	return float64(int(amount*100+0.5)) / 100
}

func AddOrUpdatePricing(modelID string, pricing PricingData) {
	pricing.ModelID = modelID
	AllPricing[modelID] = pricing
}

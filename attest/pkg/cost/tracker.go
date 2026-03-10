// pkg/cost/tracker.go - tracks API spending per model

package cost

import (
	"context"
	"database/sql"
	"fmt"
	"time"
)

// Tracker handles cost tracking and database operations
type Tracker struct {
	db *sql.DB
}

// NewTracker creates a new cost tracker
func NewTracker(db *sql.DB) *Tracker {
	return &Tracker{db: db}
}

// RecordAPICall records the cost of an API call
func (t *Tracker) RecordAPICall(ctx context.Context, model, provider string, inputTokens, outputTokens int64, runID string) (*CostEntry, error) {
	cost, err := CalculateCost(model, inputTokens, outputTokens)
	if err != nil {
		// Record with zero cost if pricing unknown
		cost = 0
	}

	pricing, _ := GetPricing(model)
	if pricing.ModelID == "" {
		pricing.Provider = provider
	}

	inputCost := (float64(inputTokens) / 1000.0) * pricing.InputPrice
	outputCost := (float64(outputTokens) / 1000.0) * pricing.OutputPrice

	now := time.Now()
	date := now.Truncate(24 * time.Hour)

	entry := &CostEntry{
		Date:         date,
		Model:        model,
		Provider:     pricing.Provider,
		InputTokens:  inputTokens,
		OutputTokens: outputTokens,
		InputCost:    inputCost,
		OutputCost:   outputCost,
		TotalCost:    cost,
		RunID:        runID,
		CreatedAt:    now,
	}

	// Insert into database
	result, err := t.db.ExecContext(ctx, `
		INSERT INTO cost_tracking (date, model, provider, input_tokens, output_tokens, 
			input_cost, output_cost, total_cost, run_id, created_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
	`, entry.Date, entry.Model, entry.Provider, entry.InputTokens, entry.OutputTokens,
		entry.InputCost, entry.OutputCost, entry.TotalCost, entry.RunID, entry.CreatedAt)

	if err != nil {
		return nil, fmt.Errorf("failed to record cost: %w", err)
	}

	entry.ID, _ = result.LastInsertId()

	// Update cumulative cost
	cumulative, err := t.GetCumulativeCost(ctx, date)
	if err != nil {
		return nil, err
	}
	entry.Cumulative = cumulative

	return entry, nil
}

// GetCumulativeCost returns total cost up to and including date
func (t *Tracker) GetCumulativeCost(ctx context.Context, date time.Time) (float64, error) {
	var cumulative float64
	err := t.db.QueryRowContext(ctx, `
		SELECT COALESCE(SUM(total_cost), 0) FROM cost_tracking WHERE date <= ?
	`, date).Scan(&cumulative)

	if err != nil {
		return 0, fmt.Errorf("failed to get cumulative cost: %w", err)
	}

	return cumulative, nil
}

// GetSpendForPeriod calculates total spend for a given time period
func (t *Tracker) GetSpendForPeriod(ctx context.Context, start, end time.Time) (float64, error) {
	var total float64
	err := t.db.QueryRowContext(ctx, `
		SELECT COALESCE(SUM(total_cost), 0) FROM cost_tracking 
		WHERE date >= ? AND date <= ?
	`, start, end).Scan(&total)

	if err != nil {
		return 0, fmt.Errorf("failed to get period spend: %w", err)
	}

	return total, nil
}

// GetDailySpend returns today's spending
func (t *Tracker) GetDailySpend(ctx context.Context) (float64, error) {
	today := time.Now().Truncate(24 * time.Hour)
	return t.GetSpendForPeriod(ctx, today, today.Add(24*time.Hour))
}

// GetWeeklySpend returns this week's spending
func (t *Tracker) GetWeeklySpend(ctx context.Context) (float64, error) {
	now := time.Now()
	weekStart := now.Truncate(24*time.Hour).AddDate(0, 0, -int(now.Weekday()))
	weekEnd := weekStart.AddDate(0, 0, 7)
	return t.GetSpendForPeriod(ctx, weekStart, weekEnd)
}

// GetMonthlySpend returns this month's spending
func (t *Tracker) GetMonthlySpend(ctx context.Context) (float64, error) {
	now := time.Now()
	monthStart := time.Date(now.Year(), now.Month(), 1, 0, 0, 0, 0, now.Location())
	monthEnd := monthStart.AddDate(0, 1, 0)
	return t.GetSpendForPeriod(ctx, monthStart, monthEnd)
}

// GetSpendStatus returns current spending for all periods
func (t *Tracker) GetSpendStatus(ctx context.Context) (*SpendStatus, error) {
	daily, err := t.GetDailySpend(ctx)
	if err != nil {
		return nil, err
	}

	weekly, err := t.GetWeeklySpend(ctx)
	if err != nil {
		return nil, err
	}

	monthly, err := t.GetMonthlySpend(ctx)
	if err != nil {
		return nil, err
	}

	return &SpendStatus{
		Daily:   daily,
		Weekly:  weekly,
		Monthly: monthly,
		AsOf:    time.Now(),
	}, nil
}

// SpendStatus aggregates spending across periods
type SpendStatus struct {
	Daily   float64
	Weekly  float64
	Monthly float64
	AsOf    time.Time
}

// GetReport generates a detailed spending report for a period
func (t *Tracker) GetReport(ctx context.Context, period BudgetPeriod) (*SpendingReport, error) {
	now := time.Now()
	var start, end time.Time

	switch period {
	case PeriodDaily:
		start = now.Truncate(24 * time.Hour)
		end = start.Add(24 * time.Hour)
	case PeriodWeekly:
		start = now.Truncate(24*time.Hour).AddDate(0, 0, -int(now.Weekday()))
		end = start.AddDate(0, 0, 7)
	case PeriodMonthly:
		start = time.Date(now.Year(), now.Month(), 1, 0, 0, 0, 0, now.Location())
		end = start.AddDate(0, 1, 0)
	default:
		return nil, fmt.Errorf("invalid period: %s", period)
	}

	// Get total spend
	total, err := t.GetSpendForPeriod(ctx, start, end)
	if err != nil {
		return nil, err
	}

	// Get breakdown by model
	byModel := make(map[string]float64)
	rows, err := t.db.QueryContext(ctx, `
		SELECT model, SUM(total_cost) FROM cost_tracking 
		WHERE date >= ? AND date <= ?
		GROUP BY model
	`, start, end)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	for rows.Next() {
		var model string
		var cost float64
		if err := rows.Scan(&model, &cost); err != nil {
			continue
		}
		byModel[model] = cost
	}

	// Get breakdown by provider
	byProvider := make(map[string]float64)
	rows, err = t.db.QueryContext(ctx, `
		SELECT provider, SUM(total_cost) FROM cost_tracking 
		WHERE date >= ? AND date <= ?
		GROUP BY provider
	`, start, end)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	for rows.Next() {
		var provider string
		var cost float64
		if err := rows.Scan(&provider, &cost); err != nil {
			continue
		}
		byProvider[provider] = cost
	}

	// Get entries
	var entries []CostEntry
	rows, err = t.db.QueryContext(ctx, `
		SELECT id, date, model, provider, input_tokens, output_tokens, 
			input_cost, output_cost, total_cost, run_id, created_at
		FROM cost_tracking 
		WHERE date >= ? AND date <= ?
		ORDER BY created_at DESC
	`, start, end)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	for rows.Next() {
		var e CostEntry
		err := rows.Scan(&e.ID, &e.Date, &e.Model, &e.Provider, &e.InputTokens, &e.OutputTokens,
			&e.InputCost, &e.OutputCost, &e.TotalCost, &e.RunID, &e.CreatedAt)
		if err != nil {
			continue
		}
		entries = append(entries, e)
	}

	return &SpendingReport{
		Period:     period,
		StartDate:  start,
		EndDate:    end,
		TotalSpent: total,
		ByModel:    byModel,
		ByProvider: byProvider,
		Entries:    entries,
	}, nil
}

// ResetCosts clears all cost tracking data (use with caution)
func (t *Tracker) ResetCosts(ctx context.Context) error {
	_, err := t.db.ExecContext(ctx, `DELETE FROM cost_tracking`)
	return err
}

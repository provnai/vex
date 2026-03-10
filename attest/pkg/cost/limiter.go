// pkg/cost/limiter.go - enforces budget limits

package cost

import (
	"context"
	"database/sql"
	"fmt"
	"log"
	"time"
)

// Limiter enforces budget constraints
type Limiter struct {
	tracker *Tracker
	db      *sql.DB
}

// NewLimiter creates a new budget limiter
func NewLimiter(db *sql.DB) *Limiter {
	return &Limiter{
		tracker: NewTracker(db),
		db:      db,
	}
}

// CheckBudget checks if any budget limits have been exceeded
// Returns error if budget exceeded and hard stop is enabled
func (l *Limiter) CheckBudget(ctx context.Context) error {
	config, err := l.GetBudgetConfig(ctx)
	if err != nil {
		return fmt.Errorf("failed to get budget config: %w", err)
	}

	status, err := l.tracker.GetSpendStatus(ctx)
	if err != nil {
		return fmt.Errorf("failed to get spend status: %w", err)
	}

	// Check daily limit
	if config.DailyLimit > 0 {
		percentage := status.Daily / config.DailyLimit
		if percentage >= 1.0 {
			if config.HardStop {
				return &BudgetExceededError{
					Period:     PeriodDaily,
					Spent:      status.Daily,
					Limit:      config.DailyLimit,
					Percentage: percentage,
				}
			}
			log.Printf("WARNING: Daily budget exceeded: %s/%s (%.1f%%)",
				FormatCurrency(status.Daily), FormatCurrency(config.DailyLimit), percentage*100)
		} else if percentage >= config.WarnThreshold {
			log.Printf("WARNING: Daily budget at %.1f%%: %s/%s",
				percentage*100, FormatCurrency(status.Daily), FormatCurrency(config.DailyLimit))
		}
	}

	// Check weekly limit
	if config.WeeklyLimit > 0 {
		percentage := status.Weekly / config.WeeklyLimit
		if percentage >= 1.0 {
			if config.HardStop {
				return &BudgetExceededError{
					Period:     PeriodWeekly,
					Spent:      status.Weekly,
					Limit:      config.WeeklyLimit,
					Percentage: percentage,
				}
			}
			log.Printf("WARNING: Weekly budget exceeded: %s/%s (%.1f%%)",
				FormatCurrency(status.Weekly), FormatCurrency(config.WeeklyLimit), percentage*100)
		} else if percentage >= config.WarnThreshold {
			log.Printf("WARNING: Weekly budget at %.1f%%: %s/%s",
				percentage*100, FormatCurrency(status.Weekly), FormatCurrency(config.WeeklyLimit))
		}
	}

	// Check monthly limit
	if config.MonthlyLimit > 0 {
		percentage := status.Monthly / config.MonthlyLimit
		if percentage >= 1.0 {
			if config.HardStop {
				return &BudgetExceededError{
					Period:     PeriodMonthly,
					Spent:      status.Monthly,
					Limit:      config.MonthlyLimit,
					Percentage: percentage,
				}
			}
			log.Printf("WARNING: Monthly budget exceeded: %s/%s (%.1f%%)",
				FormatCurrency(status.Monthly), FormatCurrency(config.MonthlyLimit), percentage*100)
		} else if percentage >= config.WarnThreshold {
			log.Printf("WARNING: Monthly budget at %.1f%%: %s/%s",
				percentage*100, FormatCurrency(status.Monthly), FormatCurrency(config.MonthlyLimit))
		}
	}

	return nil
}

// GetBudgetConfig retrieves current budget configuration
func (l *Limiter) GetBudgetConfig(ctx context.Context) (*BudgetConfig, error) {
	config := DefaultBudgetConfig()

	row := l.db.QueryRowContext(ctx, `
		SELECT id, daily_limit, weekly_limit, monthly_limit, hard_stop, warn_threshold, updated_at
		FROM budget_config LIMIT 1
	`)

	var id int64
	var updatedAt time.Time
	err := row.Scan(&id, &config.DailyLimit, &config.WeeklyLimit, &config.MonthlyLimit,
		&config.HardStop, &config.WarnThreshold, &updatedAt)

	if err == sql.ErrNoRows {
		// No config exists, create default
		return l.SetBudgetConfig(ctx, config)
	}
	if err != nil {
		return nil, fmt.Errorf("failed to get budget config: %w", err)
	}

	config.ID = id
	config.UpdatedAt = updatedAt
	return config, nil
}

// SetBudgetConfig updates budget configuration
func (l *Limiter) SetBudgetConfig(ctx context.Context, config *BudgetConfig) (*BudgetConfig, error) {
	now := time.Now()
	config.UpdatedAt = now

	// Check if config exists
	var exists bool
	err := l.db.QueryRowContext(ctx, `SELECT EXISTS(SELECT 1 FROM budget_config LIMIT 1)`).Scan(&exists)
	if err != nil {
		return nil, err
	}

	if exists {
		// Update existing
		_, err = l.db.ExecContext(ctx, `
			UPDATE budget_config 
			SET daily_limit = ?, weekly_limit = ?, monthly_limit = ?, 
			    hard_stop = ?, warn_threshold = ?, updated_at = ?
		`, config.DailyLimit, config.WeeklyLimit, config.MonthlyLimit,
			config.HardStop, config.WarnThreshold, now)
	} else {
		// Insert new
		result, err := l.db.ExecContext(ctx, `
			INSERT INTO budget_config (daily_limit, weekly_limit, monthly_limit, 
				hard_stop, warn_threshold, updated_at)
			VALUES (?, ?, ?, ?, ?, ?)
		`, config.DailyLimit, config.WeeklyLimit, config.MonthlyLimit,
			config.HardStop, config.WarnThreshold, now)
		if err == nil {
			config.ID, _ = result.LastInsertId()
		}
	}

	if err != nil {
		return nil, fmt.Errorf("failed to set budget config: %w", err)
	}

	return config, nil
}

// SetDailyLimit updates just the daily limit
func (l *Limiter) SetDailyLimit(ctx context.Context, limit float64) error {
	config, err := l.GetBudgetConfig(ctx)
	if err != nil {
		return err
	}
	config.DailyLimit = limit
	_, err = l.SetBudgetConfig(ctx, config)
	return err
}

// SetWeeklyLimit updates just the weekly limit
func (l *Limiter) SetWeeklyLimit(ctx context.Context, limit float64) error {
	config, err := l.GetBudgetConfig(ctx)
	if err != nil {
		return err
	}
	config.WeeklyLimit = limit
	_, err = l.SetBudgetConfig(ctx, config)
	return err
}

// SetMonthlyLimit updates just the monthly limit
func (l *Limiter) SetMonthlyLimit(ctx context.Context, limit float64) error {
	config, err := l.GetBudgetConfig(ctx)
	if err != nil {
		return err
	}
	config.MonthlyLimit = limit
	_, err = l.SetBudgetConfig(ctx, config)
	return err
}

// SetHardStop toggles hard stop behavior
func (l *Limiter) SetHardStop(ctx context.Context, enabled bool) error {
	config, err := l.GetBudgetConfig(ctx)
	if err != nil {
		return err
	}
	config.HardStop = enabled
	_, err = l.SetBudgetConfig(ctx, config)
	return err
}

// SetWarnThreshold updates the warning threshold (0.0-1.0)
func (l *Limiter) SetWarnThreshold(ctx context.Context, threshold float64) error {
	if threshold < 0 || threshold > 1 {
		return fmt.Errorf("threshold must be between 0.0 and 1.0")
	}

	config, err := l.GetBudgetConfig(ctx)
	if err != nil {
		return err
	}
	config.WarnThreshold = threshold
	_, err = l.SetBudgetConfig(ctx, config)
	return err
}

// PreFlightCheck performs budget check before API call
func (l *Limiter) PreFlightCheck(ctx context.Context) error {
	return l.CheckBudget(ctx)
}

// BudgetStatus provides a full status snapshot
type BudgetStatus struct {
	Config   *BudgetConfig
	Spending *SpendStatus
	Warnings []string
}

// GetFullStatus returns complete budget and spending status
func (l *Limiter) GetFullStatus(ctx context.Context) (*BudgetStatus, error) {
	config, err := l.GetBudgetConfig(ctx)
	if err != nil {
		return nil, err
	}

	spending, err := l.tracker.GetSpendStatus(ctx)
	if err != nil {
		return nil, err
	}

	var warnings []string

	if config.DailyLimit > 0 {
		pct := spending.Daily / config.DailyLimit
		if pct >= 1.0 {
			warnings = append(warnings, fmt.Sprintf("DAILY BUDGET EXCEEDED: %s/%s",
				FormatCurrency(spending.Daily), FormatCurrency(config.DailyLimit)))
		} else if pct >= config.WarnThreshold {
			warnings = append(warnings, fmt.Sprintf("Daily budget warning: %.1f%% (%s/%s)",
				pct*100, FormatCurrency(spending.Daily), FormatCurrency(config.DailyLimit)))
		}
	}

	if config.WeeklyLimit > 0 {
		pct := spending.Weekly / config.WeeklyLimit
		if pct >= 1.0 {
			warnings = append(warnings, fmt.Sprintf("WEEKLY BUDGET EXCEEDED: %s/%s",
				FormatCurrency(spending.Weekly), FormatCurrency(config.WeeklyLimit)))
		} else if pct >= config.WarnThreshold {
			warnings = append(warnings, fmt.Sprintf("Weekly budget warning: %.1f%% (%s/%s)",
				pct*100, FormatCurrency(spending.Weekly), FormatCurrency(config.WeeklyLimit)))
		}
	}

	if config.MonthlyLimit > 0 {
		pct := spending.Monthly / config.MonthlyLimit
		if pct >= 1.0 {
			warnings = append(warnings, fmt.Sprintf("MONTHLY BUDGET EXCEEDED: %s/%s",
				FormatCurrency(spending.Monthly), FormatCurrency(config.MonthlyLimit)))
		} else if pct >= config.WarnThreshold {
			warnings = append(warnings, fmt.Sprintf("Monthly budget warning: %.1f%% (%s/%s)",
				pct*100, FormatCurrency(spending.Monthly), FormatCurrency(config.MonthlyLimit)))
		}
	}

	return &BudgetStatus{
		Config:   config,
		Spending: spending,
		Warnings: warnings,
	}, nil
}

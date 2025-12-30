//! Cost Analytics Module
//!
//! Provides cost tracking, budgeting, and optimization recommendations
//! for multi-agent system operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model pricing information (USD per 1M tokens)
#[derive(Debug, Clone, PartialEq)]
pub struct ModelPricing {
    pub input_price: f64,
    pub output_price: f64,
    pub cache_read_price: f64,
    pub cache_write_price: f64,
}

impl ModelPricing {
    /// Get pricing for a specific model (as of January 2025)
    pub fn for_model(model: &str) -> Self {
        if model.contains("opus") {
            Self {
                input_price: 15.0,
                output_price: 75.0,
                cache_read_price: 1.5,    // 90% off
                cache_write_price: 18.75, // 25% premium
            }
        } else if model.contains("haiku") {
            Self {
                input_price: 0.25,
                output_price: 1.25,
                cache_read_price: 0.025,
                cache_write_price: 0.3125,
            }
        } else {
            // Sonnet (default)
            Self {
                input_price: 3.0,
                output_price: 15.0,
                cache_read_price: 0.3,
                cache_write_price: 3.75,
            }
        }
    }

    /// Calculate cost for given token counts
    pub fn calculate_cost(
        &self,
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cache_write_tokens: i64,
    ) -> f64 {
        let regular_input = input_tokens - cache_read_tokens - cache_write_tokens;
        let input_cost = (regular_input.max(0) as f64 / 1_000_000.0) * self.input_price;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_price;
        let cache_read_cost = (cache_read_tokens as f64 / 1_000_000.0) * self.cache_read_price;
        let cache_write_cost = (cache_write_tokens as f64 / 1_000_000.0) * self.cache_write_price;

        input_cost + output_cost + cache_read_cost + cache_write_cost
    }
}

/// Budget period type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BudgetPeriod {
    Daily,
    Weekly,
    Monthly,
}

impl std::fmt::Display for BudgetPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetPeriod::Daily => write!(f, "daily"),
            BudgetPeriod::Weekly => write!(f, "weekly"),
            BudgetPeriod::Monthly => write!(f, "monthly"),
        }
    }
}

impl std::str::FromStr for BudgetPeriod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "daily" => Ok(BudgetPeriod::Daily),
            "weekly" => Ok(BudgetPeriod::Weekly),
            "monthly" => Ok(BudgetPeriod::Monthly),
            _ => Err(format!("Invalid budget period: {}", s)),
        }
    }
}

/// Cost budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBudget {
    pub id: Option<i64>,
    pub period_type: BudgetPeriod,
    pub amount_usd: f64,
    pub alert_threshold_percent: i32,
    pub start_date: String, // YYYY-MM-DD
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl CostBudget {
    pub fn new(period_type: BudgetPeriod, amount_usd: f64) -> Self {
        Self {
            id: None,
            period_type,
            amount_usd,
            alert_threshold_percent: 80,
            start_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            created_at: None,
            updated_at: None,
        }
    }

    pub fn with_alert_threshold(mut self, threshold_percent: i32) -> Self {
        self.alert_threshold_percent = threshold_percent;
        self
    }

    pub fn with_start_date(mut self, start_date: String) -> Self {
        self.start_date = start_date;
        self
    }

    /// Check if budget is exceeded
    pub fn is_exceeded(&self, current_cost: f64) -> bool {
        current_cost > self.amount_usd
    }

    /// Check if alert threshold is reached
    pub fn is_alert_threshold_reached(&self, current_cost: f64) -> bool {
        let threshold = (self.amount_usd * self.alert_threshold_percent as f64) / 100.0;
        current_cost >= threshold
    }

    /// Get percentage of budget used
    pub fn percentage_used(&self, current_cost: f64) -> f64 {
        if self.amount_usd == 0.0 {
            0.0
        } else {
            (current_cost / self.amount_usd) * 100.0
        }
    }
}

/// Cost record for aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    pub date: String,
    pub entity_id: String,
    pub model: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_write_tokens: i64,
    pub request_count: i64,
    pub estimated_cost_usd: f64,
}

impl CostRecord {
    /// Calculate cost per token
    pub fn cost_per_token(&self) -> f64 {
        let total_tokens = self.total_input_tokens + self.total_output_tokens;
        if total_tokens == 0 {
            0.0
        } else {
            self.estimated_cost_usd / total_tokens as f64
        }
    }

    /// Calculate cost per request
    pub fn cost_per_request(&self) -> f64 {
        if self.request_count == 0 {
            0.0
        } else {
            self.estimated_cost_usd / self.request_count as f64
        }
    }

    /// Calculate cache efficiency (percentage of tokens from cache)
    pub fn cache_efficiency(&self) -> f64 {
        let total_input = self.total_input_tokens;
        if total_input == 0 {
            0.0
        } else {
            (self.total_cache_read_tokens as f64 / total_input as f64) * 100.0
        }
    }
}

/// Daily cost summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCost {
    pub date: String,
    pub total_cost_usd: f64,
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub models: Vec<ModelCostBreakdown>,
}

/// Cost breakdown by model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCostBreakdown {
    pub model: String,
    pub cost_usd: f64,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// Cost report with trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReport {
    pub period_start: String,
    pub period_end: String,
    pub total_cost_usd: f64,
    pub daily_costs: Vec<DailyCost>,
    pub by_agent: Vec<CostRecord>,
    pub by_epic: Vec<CostRecord>,
    pub by_story: Vec<CostRecord>,
    pub trend: CostTrend,
    pub budget_status: Option<BudgetStatus>,
}

impl CostReport {
    /// Calculate average daily cost
    pub fn avg_daily_cost(&self) -> f64 {
        if self.daily_costs.is_empty() {
            0.0
        } else {
            self.total_cost_usd / self.daily_costs.len() as f64
        }
    }

    /// Get most expensive agent
    pub fn most_expensive_agent(&self) -> Option<&CostRecord> {
        self.by_agent
            .iter()
            .max_by(|a, b| a.estimated_cost_usd.partial_cmp(&b.estimated_cost_usd).unwrap())
    }

    /// Get most expensive epic
    pub fn most_expensive_epic(&self) -> Option<&CostRecord> {
        self.by_epic
            .iter()
            .max_by(|a, b| a.estimated_cost_usd.partial_cmp(&b.estimated_cost_usd).unwrap())
    }
}

/// Cost trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTrend {
    pub direction: TrendDirection,
    pub percentage_change: f64,
    pub projected_monthly_cost: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

/// Budget status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub budget: CostBudget,
    pub current_cost: f64,
    pub percentage_used: f64,
    pub remaining: f64,
    pub is_exceeded: bool,
    pub is_alert_triggered: bool,
}

/// Cost optimization recommendation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationType {
    ModelDowngrade,
    CacheOptimization,
    PromptOptimization,
    BatchProcessing,
    RateLimiting,
}

impl std::fmt::Display for RecommendationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecommendationType::ModelDowngrade => write!(f, "model_downgrade"),
            RecommendationType::CacheOptimization => write!(f, "cache_optimization"),
            RecommendationType::PromptOptimization => write!(f, "prompt_optimization"),
            RecommendationType::BatchProcessing => write!(f, "batch_processing"),
            RecommendationType::RateLimiting => write!(f, "rate_limiting"),
        }
    }
}

impl std::str::FromStr for RecommendationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "model_downgrade" => Ok(RecommendationType::ModelDowngrade),
            "cache_optimization" => Ok(RecommendationType::CacheOptimization),
            "prompt_optimization" => Ok(RecommendationType::PromptOptimization),
            "batch_processing" => Ok(RecommendationType::BatchProcessing),
            "rate_limiting" => Ok(RecommendationType::RateLimiting),
            _ => Err(format!("Invalid recommendation type: {}", s)),
        }
    }
}

/// Entity type for recommendations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Agent,
    Epic,
    Story,
    Global,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Agent => write!(f, "agent"),
            EntityType::Epic => write!(f, "epic"),
            EntityType::Story => write!(f, "story"),
            EntityType::Global => write!(f, "global"),
        }
    }
}

impl std::str::FromStr for EntityType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "agent" => Ok(EntityType::Agent),
            "epic" => Ok(EntityType::Epic),
            "story" => Ok(EntityType::Story),
            "global" => Ok(EntityType::Global),
            _ => Err(format!("Invalid entity type: {}", s)),
        }
    }
}

/// Cost optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecommendation {
    pub id: Option<i64>,
    pub recommendation_type: RecommendationType,
    pub entity_type: EntityType,
    pub entity_id: Option<String>,
    pub description: String,
    pub potential_savings_usd: f64,
    pub confidence_score: f64,
    pub applied: bool,
    pub applied_at: Option<String>,
    pub created_at: Option<String>,
}

impl CostRecommendation {
    pub fn new(
        recommendation_type: RecommendationType,
        entity_type: EntityType,
        entity_id: Option<String>,
        description: String,
        potential_savings_usd: f64,
        confidence_score: f64,
    ) -> Self {
        Self {
            id: None,
            recommendation_type,
            entity_type,
            entity_id,
            description,
            potential_savings_usd,
            confidence_score,
            applied: false,
            applied_at: None,
            created_at: None,
        }
    }
}

/// Cost analytics engine for generating insights
pub struct CostAnalytics;

impl CostAnalytics {
    /// Calculate cost trend from daily costs
    pub fn calculate_trend(daily_costs: &[DailyCost]) -> CostTrend {
        if daily_costs.len() < 2 {
            return CostTrend {
                direction: TrendDirection::Stable,
                percentage_change: 0.0,
                projected_monthly_cost: daily_costs.first().map(|d| d.total_cost_usd * 30.0).unwrap_or(0.0),
            };
        }

        // Compare first half vs second half
        let mid = daily_costs.len() / 2;
        let first_half_avg = daily_costs[..mid].iter().map(|d| d.total_cost_usd).sum::<f64>() / mid as f64;
        let second_half_avg = daily_costs[mid..].iter().map(|d| d.total_cost_usd).sum::<f64>() / (daily_costs.len() - mid) as f64;

        let percentage_change = if first_half_avg == 0.0 {
            0.0
        } else {
            ((second_half_avg - first_half_avg) / first_half_avg) * 100.0
        };

        let direction = if percentage_change > 5.0 {
            TrendDirection::Increasing
        } else if percentage_change < -5.0 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        // Project monthly cost based on recent average
        let recent_avg = daily_costs[daily_costs.len().saturating_sub(7)..].iter()
            .map(|d| d.total_cost_usd)
            .sum::<f64>() / daily_costs.len().saturating_sub(7).max(1) as f64;
        let projected_monthly_cost = recent_avg * 30.0;

        CostTrend {
            direction,
            percentage_change,
            projected_monthly_cost,
        }
    }

    /// Generate optimization recommendations
    pub fn generate_recommendations(cost_records: &[CostRecord]) -> Vec<CostRecommendation> {
        let mut recommendations = Vec::new();

        // Analyze each cost record
        for record in cost_records {
            // Check for expensive models that could be downgraded
            if record.model.contains("opus") && record.estimated_cost_usd > 1.0 {
                let sonnet_pricing = ModelPricing::for_model("sonnet");
                let opus_pricing = ModelPricing::for_model("opus");
                let current_cost = record.estimated_cost_usd;
                let potential_cost = sonnet_pricing.calculate_cost(
                    record.total_input_tokens,
                    record.total_output_tokens,
                    record.total_cache_read_tokens,
                    record.total_cache_write_tokens,
                );
                let savings = current_cost - potential_cost;

                if savings > 0.1 {
                    recommendations.push(CostRecommendation::new(
                        RecommendationType::ModelDowngrade,
                        EntityType::Agent,
                        Some(record.entity_id.clone()),
                        format!(
                            "Consider downgrading from Opus to Sonnet for routine tasks. Estimated savings: ${:.2}/day",
                            savings
                        ),
                        savings,
                        0.8,
                    ));
                }
            }

            // Check for low cache efficiency
            if record.cache_efficiency() < 20.0 && record.total_input_tokens > 100_000 {
                let potential_savings = record.estimated_cost_usd * 0.15; // Estimate 15% savings
                recommendations.push(CostRecommendation::new(
                    RecommendationType::CacheOptimization,
                    EntityType::Agent,
                    Some(record.entity_id.clone()),
                    format!(
                        "Low cache hit rate ({:.1}%). Optimize prompt caching. Estimated savings: ${:.2}/day",
                        record.cache_efficiency(),
                        potential_savings
                    ),
                    potential_savings,
                    0.6,
                ));
            }

            // Check for high token usage per request
            let tokens_per_request = if record.request_count > 0 {
                (record.total_input_tokens + record.total_output_tokens) / record.request_count
            } else {
                0
            };

            if tokens_per_request > 10_000 {
                let potential_savings = record.estimated_cost_usd * 0.2; // Estimate 20% savings
                recommendations.push(CostRecommendation::new(
                    RecommendationType::PromptOptimization,
                    EntityType::Agent,
                    Some(record.entity_id.clone()),
                    format!(
                        "High token usage per request ({}). Optimize prompts. Estimated savings: ${:.2}/day",
                        tokens_per_request,
                        potential_savings
                    ),
                    potential_savings,
                    0.7,
                ));
            }
        }

        // Sort by potential savings
        recommendations.sort_by(|a, b| {
            b.potential_savings_usd
                .partial_cmp(&a.potential_savings_usd)
                .unwrap()
        });

        recommendations
    }

    /// Aggregate costs by model
    pub fn aggregate_by_model(cost_records: &[CostRecord]) -> HashMap<String, f64> {
        let mut by_model = HashMap::new();
        for record in cost_records {
            *by_model.entry(record.model.clone()).or_insert(0.0) += record.estimated_cost_usd;
        }
        by_model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_pricing_opus() {
        let pricing = ModelPricing::for_model("claude-opus-4");
        assert_eq!(pricing.input_price, 15.0);
        assert_eq!(pricing.output_price, 75.0);
        assert_eq!(pricing.cache_read_price, 1.5);
        assert_eq!(pricing.cache_write_price, 18.75);
    }

    #[test]
    fn test_model_pricing_sonnet() {
        let pricing = ModelPricing::for_model("claude-sonnet-3.5");
        assert_eq!(pricing.input_price, 3.0);
        assert_eq!(pricing.output_price, 15.0);
        assert_eq!(pricing.cache_read_price, 0.3);
        assert_eq!(pricing.cache_write_price, 3.75);
    }

    #[test]
    fn test_model_pricing_haiku() {
        let pricing = ModelPricing::for_model("claude-haiku-3");
        assert_eq!(pricing.input_price, 0.25);
        assert_eq!(pricing.output_price, 1.25);
        assert_eq!(pricing.cache_read_price, 0.025);
        assert_eq!(pricing.cache_write_price, 0.3125);
    }

    #[test]
    fn test_calculate_cost() {
        let pricing = ModelPricing::for_model("claude-sonnet-3.5");
        // 100K input, 50K output, 20K cache read, 10K cache write
        let cost = pricing.calculate_cost(100_000, 50_000, 20_000, 10_000);

        // Calculate expected:
        // Regular input: 100_000 - 20_000 - 10_000 = 70_000
        // Input cost: (70_000 / 1_000_000) * 3.0 = 0.21
        // Output cost: (50_000 / 1_000_000) * 15.0 = 0.75
        // Cache read: (20_000 / 1_000_000) * 0.3 = 0.006
        // Cache write: (10_000 / 1_000_000) * 3.75 = 0.0375
        // Total: 1.0035
        assert!((cost - 1.0035).abs() < 0.0001);
    }

    #[test]
    fn test_budget_period_from_str() {
        assert_eq!("daily".parse::<BudgetPeriod>().unwrap(), BudgetPeriod::Daily);
        assert_eq!("weekly".parse::<BudgetPeriod>().unwrap(), BudgetPeriod::Weekly);
        assert_eq!("monthly".parse::<BudgetPeriod>().unwrap(), BudgetPeriod::Monthly);
        assert!("invalid".parse::<BudgetPeriod>().is_err());
    }

    #[test]
    fn test_budget_period_to_string() {
        assert_eq!(BudgetPeriod::Daily.to_string(), "daily");
        assert_eq!(BudgetPeriod::Weekly.to_string(), "weekly");
        assert_eq!(BudgetPeriod::Monthly.to_string(), "monthly");
    }

    #[test]
    fn test_cost_budget_new() {
        let budget = CostBudget::new(BudgetPeriod::Monthly, 100.0);
        assert_eq!(budget.period_type, BudgetPeriod::Monthly);
        assert_eq!(budget.amount_usd, 100.0);
        assert_eq!(budget.alert_threshold_percent, 80);
    }

    #[test]
    fn test_cost_budget_is_exceeded() {
        let budget = CostBudget::new(BudgetPeriod::Monthly, 100.0);
        assert!(!budget.is_exceeded(50.0));
        assert!(!budget.is_exceeded(100.0));
        assert!(budget.is_exceeded(101.0));
    }

    #[test]
    fn test_cost_budget_alert_threshold() {
        let budget = CostBudget::new(BudgetPeriod::Monthly, 100.0);
        assert!(!budget.is_alert_threshold_reached(70.0));
        assert!(budget.is_alert_threshold_reached(80.0));
        assert!(budget.is_alert_threshold_reached(90.0));
    }

    #[test]
    fn test_cost_budget_percentage_used() {
        let budget = CostBudget::new(BudgetPeriod::Monthly, 100.0);
        assert_eq!(budget.percentage_used(50.0), 50.0);
        assert_eq!(budget.percentage_used(100.0), 100.0);
        assert_eq!(budget.percentage_used(150.0), 150.0);
    }

    #[test]
    fn test_cost_record_metrics() {
        let record = CostRecord {
            date: "2025-01-15".to_string(),
            entity_id: "agent-1".to_string(),
            model: "claude-sonnet-3.5".to_string(),
            total_input_tokens: 100_000,
            total_output_tokens: 50_000,
            total_cache_read_tokens: 20_000,
            total_cache_write_tokens: 10_000,
            request_count: 10,
            estimated_cost_usd: 1.0,
        };

        // Cost per token
        let cost_per_token = record.cost_per_token();
        assert!((cost_per_token - 1.0 / 150_000.0).abs() < 0.0000001);

        // Cost per request
        assert_eq!(record.cost_per_request(), 0.1);

        // Cache efficiency
        let efficiency = record.cache_efficiency();
        assert_eq!(efficiency, 20.0); // 20_000 / 100_000 = 20%
    }

    #[test]
    fn test_calculate_trend_stable() {
        let daily_costs = vec![
            DailyCost {
                date: "2025-01-01".to_string(),
                total_cost_usd: 10.0,
                total_requests: 100,
                total_input_tokens: 100_000,
                total_output_tokens: 50_000,
                models: vec![],
            },
            DailyCost {
                date: "2025-01-02".to_string(),
                total_cost_usd: 10.5,
                total_requests: 105,
                total_input_tokens: 105_000,
                total_output_tokens: 52_500,
                models: vec![],
            },
        ];

        let trend = CostAnalytics::calculate_trend(&daily_costs);
        assert_eq!(trend.direction, TrendDirection::Stable);
        assert!(trend.percentage_change.abs() < 10.0);
    }

    #[test]
    fn test_calculate_trend_increasing() {
        let daily_costs = vec![
            DailyCost {
                date: "2025-01-01".to_string(),
                total_cost_usd: 10.0,
                total_requests: 100,
                total_input_tokens: 100_000,
                total_output_tokens: 50_000,
                models: vec![],
            },
            DailyCost {
                date: "2025-01-02".to_string(),
                total_cost_usd: 20.0,
                total_requests: 200,
                total_input_tokens: 200_000,
                total_output_tokens: 100_000,
                models: vec![],
            },
        ];

        let trend = CostAnalytics::calculate_trend(&daily_costs);
        assert_eq!(trend.direction, TrendDirection::Increasing);
        assert!(trend.percentage_change > 5.0);
    }

    #[test]
    fn test_generate_recommendations_model_downgrade() {
        let records = vec![CostRecord {
            date: "2025-01-15".to_string(),
            entity_id: "agent-1".to_string(),
            model: "claude-opus-4".to_string(),
            total_input_tokens: 100_000,
            total_output_tokens: 50_000,
            total_cache_read_tokens: 20_000,
            total_cache_write_tokens: 10_000,
            request_count: 10,
            estimated_cost_usd: 10.0,
        }];

        let recommendations = CostAnalytics::generate_recommendations(&records);
        assert!(!recommendations.is_empty());

        let has_downgrade = recommendations.iter().any(|r| {
            r.recommendation_type == RecommendationType::ModelDowngrade
        });
        assert!(has_downgrade);
    }

    #[test]
    fn test_generate_recommendations_cache_optimization() {
        let records = vec![CostRecord {
            date: "2025-01-15".to_string(),
            entity_id: "agent-1".to_string(),
            model: "claude-sonnet-3.5".to_string(),
            total_input_tokens: 200_000,
            total_output_tokens: 100_000,
            total_cache_read_tokens: 10_000, // Only 5% cache hit rate
            total_cache_write_tokens: 5_000,
            request_count: 10,
            estimated_cost_usd: 5.0,
        }];

        let recommendations = CostAnalytics::generate_recommendations(&records);

        let has_cache = recommendations.iter().any(|r| {
            r.recommendation_type == RecommendationType::CacheOptimization
        });
        assert!(has_cache);
    }

    #[test]
    fn test_generate_recommendations_prompt_optimization() {
        let records = vec![CostRecord {
            date: "2025-01-15".to_string(),
            entity_id: "agent-1".to_string(),
            model: "claude-sonnet-3.5".to_string(),
            total_input_tokens: 500_000, // 50K tokens per request
            total_output_tokens: 100_000,
            total_cache_read_tokens: 50_000,
            total_cache_write_tokens: 25_000,
            request_count: 10,
            estimated_cost_usd: 5.0,
        }];

        let recommendations = CostAnalytics::generate_recommendations(&records);

        let has_prompt = recommendations.iter().any(|r| {
            r.recommendation_type == RecommendationType::PromptOptimization
        });
        assert!(has_prompt);
    }

    #[test]
    fn test_aggregate_by_model() {
        let records = vec![
            CostRecord {
                date: "2025-01-15".to_string(),
                entity_id: "agent-1".to_string(),
                model: "claude-sonnet-3.5".to_string(),
                total_input_tokens: 100_000,
                total_output_tokens: 50_000,
                total_cache_read_tokens: 20_000,
                total_cache_write_tokens: 10_000,
                request_count: 10,
                estimated_cost_usd: 1.0,
            },
            CostRecord {
                date: "2025-01-15".to_string(),
                entity_id: "agent-2".to_string(),
                model: "claude-sonnet-3.5".to_string(),
                total_input_tokens: 100_000,
                total_output_tokens: 50_000,
                total_cache_read_tokens: 20_000,
                total_cache_write_tokens: 10_000,
                request_count: 10,
                estimated_cost_usd: 1.0,
            },
            CostRecord {
                date: "2025-01-15".to_string(),
                entity_id: "agent-3".to_string(),
                model: "claude-opus-4".to_string(),
                total_input_tokens: 50_000,
                total_output_tokens: 25_000,
                total_cache_read_tokens: 10_000,
                total_cache_write_tokens: 5_000,
                request_count: 5,
                estimated_cost_usd: 5.0,
            },
        ];

        let by_model = CostAnalytics::aggregate_by_model(&records);
        assert_eq!(by_model.len(), 2);
        assert_eq!(by_model.get("claude-sonnet-3.5"), Some(&2.0));
        assert_eq!(by_model.get("claude-opus-4"), Some(&5.0));
    }

    #[test]
    fn test_cost_report_avg_daily_cost() {
        let report = CostReport {
            period_start: "2025-01-01".to_string(),
            period_end: "2025-01-03".to_string(),
            total_cost_usd: 30.0,
            daily_costs: vec![
                DailyCost {
                    date: "2025-01-01".to_string(),
                    total_cost_usd: 10.0,
                    total_requests: 100,
                    total_input_tokens: 100_000,
                    total_output_tokens: 50_000,
                    models: vec![],
                },
                DailyCost {
                    date: "2025-01-02".to_string(),
                    total_cost_usd: 10.0,
                    total_requests: 100,
                    total_input_tokens: 100_000,
                    total_output_tokens: 50_000,
                    models: vec![],
                },
                DailyCost {
                    date: "2025-01-03".to_string(),
                    total_cost_usd: 10.0,
                    total_requests: 100,
                    total_input_tokens: 100_000,
                    total_output_tokens: 50_000,
                    models: vec![],
                },
            ],
            by_agent: vec![],
            by_epic: vec![],
            by_story: vec![],
            trend: CostTrend {
                direction: TrendDirection::Stable,
                percentage_change: 0.0,
                projected_monthly_cost: 300.0,
            },
            budget_status: None,
        };

        assert_eq!(report.avg_daily_cost(), 10.0);
    }

    #[test]
    fn test_cost_report_most_expensive() {
        let report = CostReport {
            period_start: "2025-01-01".to_string(),
            period_end: "2025-01-03".to_string(),
            total_cost_usd: 30.0,
            daily_costs: vec![],
            by_agent: vec![
                CostRecord {
                    date: "2025-01-15".to_string(),
                    entity_id: "agent-1".to_string(),
                    model: "claude-sonnet-3.5".to_string(),
                    total_input_tokens: 100_000,
                    total_output_tokens: 50_000,
                    total_cache_read_tokens: 20_000,
                    total_cache_write_tokens: 10_000,
                    request_count: 10,
                    estimated_cost_usd: 5.0,
                },
                CostRecord {
                    date: "2025-01-15".to_string(),
                    entity_id: "agent-2".to_string(),
                    model: "claude-opus-4".to_string(),
                    total_input_tokens: 50_000,
                    total_output_tokens: 25_000,
                    total_cache_read_tokens: 10_000,
                    total_cache_write_tokens: 5_000,
                    request_count: 5,
                    estimated_cost_usd: 15.0,
                },
            ],
            by_epic: vec![],
            by_story: vec![],
            trend: CostTrend {
                direction: TrendDirection::Stable,
                percentage_change: 0.0,
                projected_monthly_cost: 300.0,
            },
            budget_status: None,
        };

        let most_expensive = report.most_expensive_agent().unwrap();
        assert_eq!(most_expensive.entity_id, "agent-2");
        assert_eq!(most_expensive.estimated_cost_usd, 15.0);
    }
}

# Threshold Monitoring for Circuit Breaker

## Overview

The threshold monitoring system provides automatic circuit breaker triggers based on configurable thresholds for failure rates and token outflow volumes. This feature enhances the existing circuit breaker by detecting and responding to abnormal patterns automatically, without requiring manual intervention.

## Features

- **Automatic Threshold Detection**: Monitors failure rates and outflow volumes in sliding time windows
- **Configurable Thresholds**: Administrators can tune thresholds based on risk profile
- **Anti-Flapping**: Cooldown periods with exponential backoff prevent rapid state oscillation
- **Comprehensive Events**: Detailed events for all threshold breaches and configuration changes
- **Manual Override**: Administrators can manually reset metrics when needed

## Configuration

### Threshold Parameters

```rust
pub struct ThresholdConfig {
    pub failure_rate_threshold: u32,      // Max failures per window (1-1000)
    pub outflow_volume_threshold: i128,   // Max outflow per window (> 0)
    pub max_single_payout: i128,          // Max single transaction (> 0)
    pub time_window_secs: u64,            // Window duration (10-86400 seconds)
    pub cooldown_period_secs: u64,        // Cooldown before reopening (60-3600 seconds)
    pub cooldown_multiplier: u32,         // Backoff multiplier for repeated breaches
}
```

### Configuration Profiles

#### Conservative (High Security)
```rust
ThresholdConfig {
    failure_rate_threshold: 5,
    outflow_volume_threshold: 1_000_000_0000000,  // 1M tokens
    max_single_payout: 100_000_0000000,           // 100K tokens
    time_window_secs: 300,                        // 5 minutes
    cooldown_period_secs: 600,                    // 10 minutes
    cooldown_multiplier: 2,
}
```

#### Balanced (Moderate Security)
```rust
ThresholdConfig {
    failure_rate_threshold: 10,
    outflow_volume_threshold: 5_000_000_0000000,  // 5M tokens
    max_single_payout: 500_000_0000000,           // 500K tokens
    time_window_secs: 600,                        // 10 minutes
    cooldown_period_secs: 300,                    // 5 minutes
    cooldown_multiplier: 2,
}
```

#### Permissive (High Availability)
```rust
ThresholdConfig {
    failure_rate_threshold: 50,
    outflow_volume_threshold: 50_000_000_0000000, // 50M tokens
    max_single_payout: 5_000_000_0000000,         // 5M tokens
    time_window_secs: 3600,                       // 1 hour
    cooldown_period_secs: 180,                    // 3 minutes
    cooldown_multiplier: 1,
}
```

## Usage

### Initialization

```rust
// Initialize threshold monitoring (one-time setup)
client.init_threshold_monitoring();
```

### Configuration

```rust
// Configure thresholds (admin only)
let config = ThresholdConfig {
    failure_rate_threshold: 10,
    outflow_volume_threshold: 5_000_000_0000000,
    max_single_payout: 500_000_0000000,
    time_window_secs: 600,
    cooldown_period_secs: 300,
    cooldown_multiplier: 2,
};

client.configure_thresholds(&admin, &config);
```

### Monitoring

```rust
// Get current configuration
let config = client.get_threshold_config();

// Get current metrics
let metrics = client.get_threshold_status();
println!("Failures: {}", metrics.failure_count);
println!("Total outflow: {}", metrics.total_outflow);
println!("Window start: {}", metrics.window_start);
```

### Manual Reset

```rust
// Reset metrics (admin only)
client.reset_threshold_metrics(&admin);
```

## How It Works

### Sliding Time Windows

Metrics are tracked in sliding time windows. When a window expires:
1. Current metrics are archived
2. A new window starts with zero counters
3. A window rotation event is emitted

### Threshold Checking

Before each protected operation:
1. Check if circuit breaker is open (existing logic)
2. Check if any thresholds are breached (new logic)
3. If breached, open circuit and apply cooldown
4. If OK, proceed with operation

After each operation:
- Record success/failure metrics
- Record outflow amounts
- Update window statistics

### Cooldown and Anti-Flapping

When a threshold is breached:
1. Circuit opens immediately
2. Cooldown period is applied
3. If circuit reopens and breaches again, cooldown multiplier increases
4. After stability period, multiplier resets to base value

## Events

### Threshold Breach
```
Topic: ("th_breach", metric_type)
Data: (threshold_value, actual_value, timestamp, breach_count)
```

### Configuration Update
```
Topic: ("th_cfg", "update")
Data: (prev_failure_threshold, new_failure_threshold, prev_outflow_threshold, new_outflow_threshold)
```

### Window Rotation
```
Topic: ("th_win", "rotate")
Data: (window_start, failure_count, success_count, total_outflow)
```

### Metrics Reset
```
Topic: ("th_reset",)
Data: (admin_address, timestamp)
```

## Tuning Guidance

### Start Conservative
Begin with strict thresholds and relax based on observed patterns. It's easier to loosen restrictions than to recover from an exploit.

### Monitor False Positives
Track legitimate operations that trigger thresholds. If false positives are frequent, adjust thresholds upward.

### Adjust Window Size
- **Larger windows** (30-60 minutes): Smooth out spikes, less reactive
- **Smaller windows** (5-10 minutes): React faster, more sensitive

### Balance Cooldown
- **Longer cooldowns** (10-30 minutes): Prevent flapping, slower recovery
- **Shorter cooldowns** (3-5 minutes): Faster recovery, risk of flapping

### Review Breach Logs
Analyze breach patterns to identify:
- Legitimate usage spikes that need higher thresholds
- Attack patterns that indicate security issues
- System bugs causing excessive failures

## Troubleshooting

### Circuit Opens Frequently
- Check if thresholds are too strict for normal usage
- Review breach events to identify patterns
- Consider increasing time window or thresholds

### Circuit Doesn't Open During Attack
- Thresholds may be too permissive
- Check if metrics are being recorded correctly
- Verify threshold configuration is active

### Metrics Not Updating
- Ensure threshold monitoring is initialized
- Check that operations are calling record functions
- Verify window hasn't expired (check window_start)

### Cooldown Too Long
- Reduce cooldown_period_secs
- Set cooldown_multiplier to 1 to disable exponential backoff
- Use manual reset to force transition to HalfOpen

## Security Considerations

1. **Admin Authorization**: All configuration and reset operations require circuit breaker admin authentication
2. **Threshold Bounds**: Configuration validation enforces reasonable bounds on all values
3. **Cooldown Enforcement**: Mandatory cooldown periods prevent rapid state oscillation
4. **Metric Integrity**: Metrics cannot be manipulated by non-admin users
5. **Overflow Protection**: All arithmetic uses checked operations to prevent overflow

## Integration with Existing Circuit Breaker

The threshold monitor extends but does not replace the existing circuit breaker:

- **Existing logic**: Tracks consecutive failures, manual admin control
- **New logic**: Tracks metrics in time windows, automatic threshold triggers
- **Combined**: Both systems can open the circuit; either can trigger protection

This layered approach provides defense in depth against different types of failures.

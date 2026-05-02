use chrono::{DateTime, Duration, Utc};

/// Configures the dice-roll cooldown behaviour.
#[derive(Debug, Clone)]
pub struct CooldownConfig {
    /// Minimum time that must pass between dice rolls.
    pub duration: Duration,
    /// If true, the cooldown always resets at midnight UTC regardless of
    /// elapsed duration. With this enabled a player who rolls at 23:00 can
    /// roll again at 00:00.
    pub reset_at_midnight: bool,
}

impl Default for CooldownConfig {
    fn default() -> Self {
        CooldownConfig {
            duration: Duration::hours(24),
            reset_at_midnight: true,
        }
    }
}

impl CooldownConfig {
    /// Returns `true` if the player is still on cooldown.
    ///
    /// A player passes the cooldown check if **either**:
    /// - midnight has passed since their last roll (and `reset_at_midnight` is on), OR
    /// - the configured `duration` has elapsed since their last roll.
    pub fn is_on_cooldown(&self, last_roll_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
        if self.reset_at_midnight {
            let today_midnight = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
            if last_roll_at < today_midnight {
                return false;
            }
        }
        now - last_roll_at < self.duration
    }

    /// Convenience: returns `true` if the player has never rolled.
    pub fn is_first_roll(&self, last_roll_at: Option<DateTime<Utc>>) -> bool {
        last_roll_at.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn dt(day: u32, hour: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, day, hour, min, 0)
            .single()
            .unwrap()
    }

    #[test]
    fn test_midnight_reset_allows_next_day() {
        let config = CooldownConfig::default(); // 24h, midnight
                                                // Rolled yesterday at 13:00, now it's 07:00 next day
        assert!(!config.is_on_cooldown(dt(28, 13, 0), dt(29, 7, 0)));
    }

    #[test]
    fn test_midnight_reset_blocks_same_day() {
        let config = CooldownConfig::default();
        // Rolled at 07:00, now it's 10:00 same day
        assert!(config.is_on_cooldown(dt(28, 7, 0), dt(28, 10, 0)));
    }

    #[test]
    fn test_duration_only_12h_allows_after_elapsed() {
        let config = CooldownConfig {
            duration: Duration::hours(12),
            reset_at_midnight: false,
        };
        // Rolled at 13:00, now it's 01:00 next day (12h+)
        assert!(!config.is_on_cooldown(dt(28, 13, 0), dt(29, 1, 0)));
    }

    #[test]
    fn test_duration_only_12h_blocks_before_elapsed() {
        let config = CooldownConfig {
            duration: Duration::hours(12),
            reset_at_midnight: false,
        };
        // Rolled at 13:00, now it's 20:00 same day (7h, not enough)
        assert!(config.is_on_cooldown(dt(28, 13, 0), dt(28, 20, 0)));
    }

    #[test]
    fn test_12h_with_midnight_midnight_wins() {
        let config = CooldownConfig {
            duration: Duration::hours(12),
            reset_at_midnight: true,
        };
        // Rolled at 13:00, now it's 07:00 next day — midnight passed
        assert!(!config.is_on_cooldown(dt(28, 13, 0), dt(29, 7, 0)));
    }

    #[test]
    fn test_12h_with_midnight_duration_wins_when_sooner() {
        let config = CooldownConfig {
            duration: Duration::hours(12),
            reset_at_midnight: true,
        };
        // Rolled at 02:00, now it's 16:00 same day — 14h elapsed, no midnight yet
        assert!(!config.is_on_cooldown(dt(28, 2, 0), dt(28, 16, 0)));
    }

    #[test]
    fn test_first_roll_never_on_cooldown() {
        let config = CooldownConfig::default();
        assert!(config.is_first_roll(None));
    }
}

//! Gantt diagram types

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

/// Duration unit for task durations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DurationUnit {
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
}

impl DurationUnit {
    /// Parse a duration unit from a string suffix
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ms" => Some(Self::Milliseconds),
            "s" => Some(Self::Seconds),
            "m" => Some(Self::Minutes),
            "h" => Some(Self::Hours),
            "d" => Some(Self::Days),
            "w" => Some(Self::Weeks),
            _ => None,
        }
    }
}

/// A parsed duration value
#[derive(Debug, Clone, PartialEq)]
pub struct Duration {
    pub value: f64,
    pub unit: DurationUnit,
}

impl Duration {
    /// Create a new duration
    pub fn new(value: f64, unit: DurationUnit) -> Self {
        Self { value, unit }
    }

    /// Convert duration to milliseconds
    pub fn to_millis(&self) -> i64 {
        let base = match self.unit {
            DurationUnit::Milliseconds => 1.0,
            DurationUnit::Seconds => 1000.0,
            DurationUnit::Minutes => 60.0 * 1000.0,
            DurationUnit::Hours => 60.0 * 60.0 * 1000.0,
            DurationUnit::Days => 24.0 * 60.0 * 60.0 * 1000.0,
            DurationUnit::Weeks => 7.0 * 24.0 * 60.0 * 60.0 * 1000.0,
        };
        (self.value * base) as i64
    }
}

/// Task status flags
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskFlags {
    pub active: bool,
    pub done: bool,
    pub critical: bool,
    pub milestone: bool,
    pub vert: bool,
}

/// A task in the Gantt chart
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub task: String,
    pub section: String,
    pub order: usize,
    pub start_time: Option<NaiveDateTime>,
    pub end_time: Option<NaiveDateTime>,
    pub render_end_time: Option<NaiveDateTime>,
    pub manual_end_time: bool,
    pub flags: TaskFlags,
    /// Dependencies - task IDs this task depends on (starts after)
    pub after: Vec<String>,
    /// Dependencies - task IDs this task must end before
    pub until: Vec<String>,
    /// Raw duration string if specified
    pub raw_duration: Option<String>,
    /// Raw end date string if specified
    pub raw_end: Option<String>,
}

impl Task {
    /// Create a new task with default values
    pub fn new(id: String, task: String, section: String, order: usize) -> Self {
        Self {
            id,
            task,
            section,
            order,
            start_time: None,
            end_time: None,
            render_end_time: None,
            manual_end_time: false,
            flags: TaskFlags::default(),
            after: Vec::new(),
            until: Vec::new(),
            raw_duration: None,
            raw_end: None,
        }
    }
}

/// Which day the weekend starts on
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WeekendStart {
    #[default]
    Saturday,
    Friday,
}

/// Display mode for the Gantt chart
#[derive(Debug, Clone, PartialEq, Default)]
pub enum DisplayMode {
    #[default]
    Normal,
    Compact,
}

impl DisplayMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DisplayMode::Normal => "",
            DisplayMode::Compact => "compact",
        }
    }
}

/// The Gantt database that stores all diagram data
#[derive(Debug, Clone, Default)]
pub struct GanttDb {
    /// Diagram title
    pub title: String,
    /// Accessibility title
    pub acc_title: String,
    /// Accessibility description
    pub acc_description: String,
    /// Date format string (dayjs/moment format)
    pub date_format: String,
    /// Axis format string
    pub axis_format: String,
    /// Tick interval
    pub tick_interval: String,
    /// Today marker setting ("off" or a style string)
    pub today_marker: String,
    /// Current section name
    pub current_section: String,
    /// All sections in order
    pub sections: Vec<String>,
    /// All tasks
    pub tasks: Vec<Task>,
    /// Excluded dates/weekends
    pub excludes: Vec<String>,
    /// Whether to include weekends in excludes
    pub exclude_weekends: bool,
    /// Which day the weekend starts
    pub weekend_start: WeekendStart,
    /// Whether end dates are inclusive
    pub inclusive_end_dates: bool,
    /// Display mode
    pub display_mode: DisplayMode,
    /// Counter for auto-generated task IDs
    task_counter: usize,
}

impl GanttDb {
    /// Create a new empty GanttDb
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Set the date format
    pub fn set_date_format(&mut self, format: &str) {
        self.date_format = format.to_string();
    }

    /// Get the date format
    pub fn get_date_format(&self) -> &str {
        &self.date_format
    }

    /// Set the axis format
    pub fn set_axis_format(&mut self, format: &str) {
        self.axis_format = format.to_string();
    }

    /// Get the axis format
    pub fn get_axis_format(&self) -> &str {
        &self.axis_format
    }

    /// Set the accessibility title
    pub fn set_acc_title(&mut self, title: &str) {
        self.acc_title = title.to_string();
    }

    /// Get the accessibility title
    pub fn get_acc_title(&self) -> &str {
        &self.acc_title
    }

    /// Set the accessibility description
    pub fn set_acc_description(&mut self, desc: &str) {
        self.acc_description = desc.to_string();
    }

    /// Get the accessibility description
    pub fn get_acc_description(&self) -> &str {
        &self.acc_description
    }

    /// Enable inclusive end dates
    pub fn enable_inclusive_end_dates(&mut self) {
        self.inclusive_end_dates = true;
    }

    /// Check if end dates are inclusive
    pub fn end_dates_are_inclusive(&self) -> bool {
        self.inclusive_end_dates
    }

    /// Set the display mode
    pub fn set_display_mode(&mut self, mode: &str) {
        self.display_mode = match mode {
            "compact" => DisplayMode::Compact,
            _ => DisplayMode::Normal,
        };
    }

    /// Get the display mode
    pub fn get_display_mode(&self) -> &str {
        self.display_mode.as_str()
    }

    /// Set the today marker
    pub fn set_today_marker(&mut self, marker: &str) {
        self.today_marker = marker.to_string();
    }

    /// Get the today marker
    pub fn get_today_marker(&self) -> &str {
        &self.today_marker
    }

    /// Set which day the weekend starts
    pub fn set_weekend(&mut self, day: &str) {
        self.weekend_start = match day.to_lowercase().as_str() {
            "friday" => WeekendStart::Friday,
            _ => WeekendStart::Saturday,
        };
    }

    /// Get which day the weekend starts
    pub fn get_weekend(&self) -> WeekendStart {
        self.weekend_start
    }

    /// Set the tick interval
    pub fn set_tick_interval(&mut self, interval: &str) {
        self.tick_interval = interval.to_string();
    }

    /// Get the tick interval
    pub fn get_tick_interval(&self) -> &str {
        &self.tick_interval
    }

    /// Set top axis mode
    pub fn set_top_axis(&mut self, enabled: bool) {
        // For now just track if it's set - rendering will use this
        if enabled {
            // Store as a flag in title or use a separate field
        }
    }

    /// Set the diagram title
    pub fn set_diagram_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Get the diagram title
    pub fn get_diagram_title(&self) -> &str {
        &self.title
    }

    /// Set includes (dates to include)
    pub fn set_includes(&mut self, includes: &str) {
        // Parse includes similar to excludes
        for part in includes.split([',', ' ']) {
            let part = part.trim();
            if !part.is_empty() {
                // For now, includes would cancel out excludes
            }
        }
    }

    /// Set weekday start
    pub fn set_weekday(&mut self, _day: &str) {
        // For future use - sets which day is considered first day of week
    }

    /// Set a link on a task
    pub fn set_link(&mut self, _task_id: &str, _href: &str) {
        // Store link for task - would need to add to Task struct
    }

    /// Set a click event on a task
    pub fn set_click_event(&mut self, _task_id: &str, _callback: &str, _args: Option<&str>) {
        // Store click handler for task - would need to add to Task struct
    }

    /// Set excludes (weekends, specific dates)
    pub fn set_excludes(&mut self, excludes: &str) {
        self.excludes.clear();
        for part in excludes.split([',', ' ']) {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if part == "weekends" {
                self.exclude_weekends = true;
            } else {
                self.excludes.push(part.to_string());
            }
        }
    }

    /// Get the excludes list
    pub fn get_excludes(&self) -> &[String] {
        &self.excludes
    }

    /// Add a section
    pub fn add_section(&mut self, name: &str) {
        self.current_section = name.to_string();
        if !self.sections.contains(&self.current_section) {
            self.sections.push(self.current_section.clone());
        }
    }

    /// Get all sections
    pub fn get_sections(&self) -> &[String] {
        &self.sections
    }

    /// Generate the next task ID
    fn next_task_id(&mut self) -> String {
        self.task_counter += 1;
        format!("task{}", self.task_counter)
    }

    /// Add a task
    pub fn add_task(&mut self, name: &str, data: &str) {
        let order = self.tasks.len();
        let section = self.current_section.clone();

        // Parse the task data
        let parsed = self.parse_task_data(name, data);

        let mut task = Task::new(
            parsed.id.unwrap_or_else(|| self.next_task_id()),
            name.to_string(),
            section,
            order,
        );

        task.flags = parsed.flags;
        task.after = parsed.after;
        task.until = parsed.until;
        task.raw_duration = parsed.duration;
        task.raw_end = parsed.end_date;

        // Try to parse start time
        if let Some(start) = &parsed.start_date {
            task.start_time = self.parse_date(start);
        }

        self.tasks.push(task);
    }

    /// Get all tasks (with resolved dependencies)
    pub fn get_tasks(&mut self) -> Vec<Task> {
        self.resolve_dependencies();
        self.tasks.clone()
    }

    /// Parse a duration string like "1d", "2w", "1ms"
    pub fn parse_duration(&self, s: &str) -> (f64, &'static str) {
        // Try to find the unit suffix
        let s = s.trim();

        // Check for multi-char units first
        for (suffix, unit) in [
            ("ms", "ms"),
            ("w", "w"),
            ("d", "d"),
            ("h", "h"),
            ("m", "m"),
            ("s", "s"),
        ] {
            if let Some(num_str) = s.strip_suffix(suffix) {
                if let Ok(value) = num_str.parse::<f64>() {
                    return (value, unit);
                }
            }
        }

        // Unknown unit - return NaN with ms
        (f64::NAN, "ms")
    }

    /// Parse a date string according to the current date format
    fn parse_date(&self, s: &str) -> Option<NaiveDateTime> {
        let s = s.trim();

        // Handle millisecond timestamp format
        if self.date_format == "x" {
            if let Ok(ms) = s.parse::<i64>() {
                return DateTime::<Utc>::from_timestamp_millis(ms).map(|dt| dt.naive_utc());
            }
            return None;
        }

        // Handle seconds-only format
        if self.date_format == "ss" {
            if let Ok(secs) = s.parse::<i64>() {
                return DateTime::<Utc>::from_timestamp(secs, 0).map(|dt| dt.naive_utc());
            }
            return None;
        }

        // Convert dayjs/moment format to chrono format
        let chrono_format = self.convert_format(&self.date_format);

        // Try parsing as datetime first, then as date
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, &chrono_format) {
            return Some(dt);
        }

        if let Ok(date) = NaiveDate::parse_from_str(s, &chrono_format) {
            return date.and_hms_opt(0, 0, 0);
        }

        None
    }

    /// Convert dayjs/moment format to chrono format
    fn convert_format(&self, format: &str) -> String {
        format
            .replace("YYYY", "%Y")
            .replace("MM", "%m")
            .replace("DD", "%d")
            .replace("HH", "%H")
            .replace("mm", "%M")
            .replace("ss", "%S")
    }

    /// Resolve task dependencies and compute dates
    fn resolve_dependencies(&mut self) {
        // Build a map of task id -> index
        let id_to_idx: std::collections::HashMap<String, usize> = self
            .tasks
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id.clone(), i))
            .collect();

        // We need to iterate multiple times to resolve dependencies
        // since tasks can depend on tasks that come later in the list
        let max_iterations = self.tasks.len() + 1;

        for _ in 0..max_iterations {
            let mut changed = false;

            for i in 0..self.tasks.len() {
                // Get the task's after dependencies
                let after_ids = self.tasks[i].after.clone();
                let until_ids = self.tasks[i].until.clone();

                // Resolve "after" dependencies - start after the latest end time
                if !after_ids.is_empty() && self.tasks[i].start_time.is_none() {
                    let mut latest_end: Option<NaiveDateTime> = None;
                    let mut all_resolved = true;

                    for dep_id in &after_ids {
                        if let Some(&dep_idx) = id_to_idx.get(dep_id) {
                            if let Some(end) = self.tasks[dep_idx].end_time {
                                latest_end = Some(match latest_end {
                                    Some(current) => current.max(end),
                                    None => end,
                                });
                            } else {
                                all_resolved = false;
                            }
                        } else {
                            // Unknown dependency - use today
                            let today = chrono::Local::now()
                                .naive_local()
                                .date()
                                .and_hms_opt(0, 0, 0);
                            latest_end = Some(match latest_end {
                                Some(current) => today.map(|t| current.max(t)).unwrap_or(current),
                                None => today.unwrap_or_default(),
                            });
                        }
                    }

                    if all_resolved {
                        if let Some(start) = latest_end {
                            self.tasks[i].start_time = Some(start);
                            changed = true;
                        }
                    }
                }

                // If no start time yet and no after deps, use previous task's end
                if self.tasks[i].start_time.is_none() && after_ids.is_empty() && i > 0 {
                    // Find the previous task in the same section or overall
                    if let Some(prev_end) = self.tasks[i - 1].end_time {
                        self.tasks[i].start_time = Some(prev_end);
                        changed = true;
                    }
                }

                // Calculate end time based on duration
                if self.tasks[i].end_time.is_none() {
                    if let Some(start) = self.tasks[i].start_time {
                        if let Some(ref dur_str) = self.tasks[i].raw_duration.clone() {
                            let (value, unit) = self.parse_duration(dur_str);
                            if !value.is_nan() {
                                let end = self.add_duration(start, value, unit);
                                self.tasks[i].end_time = Some(end);
                                // Set render_end_time for excludes handling
                                self.tasks[i].render_end_time = Some(end);
                                changed = true;
                            }
                        } else if let Some(ref end_str) = self.tasks[i].raw_end.clone() {
                            // Try to parse as date
                            if let Some(end) = self.parse_date(end_str) {
                                let mut final_end = end;
                                if self.inclusive_end_dates {
                                    final_end = self.add_duration(end, 1.0, "d");
                                }
                                self.tasks[i].end_time = Some(final_end);
                                self.tasks[i].manual_end_time = true;
                                // render_end_time is None for fixed ends
                                changed = true;
                            }
                        }
                    }
                }

                // Resolve "until" dependencies - end before the earliest start time
                if !until_ids.is_empty() && self.tasks[i].end_time.is_none() {
                    let mut earliest_start: Option<NaiveDateTime> = None;

                    for dep_id in &until_ids {
                        if let Some(&dep_idx) = id_to_idx.get(dep_id) {
                            if let Some(start) = self.tasks[dep_idx].start_time {
                                earliest_start = Some(match earliest_start {
                                    Some(current) => current.min(start),
                                    None => start,
                                });
                            }
                        }
                    }

                    if let Some(end) = earliest_start {
                        self.tasks[i].end_time = Some(end);
                        changed = true;
                    }
                }
            }

            if !changed {
                break;
            }
        }

        // Apply weekend/exclude handling for render times
        if self.exclude_weekends || !self.excludes.is_empty() {
            self.apply_excludes();
        }
    }

    /// Add a duration to a datetime
    fn add_duration(&self, dt: NaiveDateTime, value: f64, unit: &str) -> NaiveDateTime {
        use chrono::Duration;

        match unit {
            "ms" => dt + Duration::milliseconds((value) as i64),
            "s" => dt + Duration::milliseconds((value * 1000.0) as i64),
            "m" => dt + Duration::minutes(value as i64),
            "h" => dt + Duration::hours(value as i64),
            "d" => dt + Duration::days(value as i64),
            "w" => dt + Duration::weeks(value as i64),
            _ => dt,
        }
    }

    /// Apply weekend and exclude handling
    fn apply_excludes(&mut self) {
        // For now, just skip weekends when calculating render times
        // This is a simplified implementation
        for task in &mut self.tasks {
            if task.manual_end_time {
                // Fixed end times don't get render_end_time
                task.render_end_time = None;
            }
        }
    }

    /// Parse task data string
    fn parse_task_data(&self, _name: &str, data: &str) -> ParsedTaskData {
        let mut result = ParsedTaskData::default();

        let parts: Vec<&str> = data.split(',').map(|s| s.trim()).collect();

        let mut idx = 0;

        // Parse flags and id from the beginning
        while idx < parts.len() {
            let part = parts[idx];
            match part {
                "done" => result.flags.done = true,
                "active" => result.flags.active = true,
                "crit" => result.flags.critical = true,
                "milestone" => result.flags.milestone = true,
                "vert" => result.flags.vert = true,
                _ => {
                    // Check if it's an "after" reference
                    if let Some(stripped) = part.strip_prefix("after ") {
                        let deps = stripped.split_whitespace().map(|s| s.to_string()).collect();
                        result.after = deps;
                    } else if let Some(stripped) = part.strip_prefix("until ") {
                        let deps = stripped.split_whitespace().map(|s| s.to_string()).collect();
                        result.until = deps;
                    } else if self.looks_like_date(part) {
                        // It's a date
                        if result.start_date.is_none() {
                            result.start_date = Some(part.to_string());
                        } else if result.end_date.is_none() {
                            result.end_date = Some(part.to_string());
                        }
                    } else if self.looks_like_duration(part) {
                        result.duration = Some(part.to_string());
                    } else if result.id.is_none() {
                        // Must be an ID
                        result.id = Some(part.to_string());
                    }
                }
            }
            idx += 1;
        }

        result
    }

    /// Check if a string looks like a date
    fn looks_like_date(&self, s: &str) -> bool {
        // Check for common date patterns
        if self.date_format == "x" {
            // Millisecond timestamp - just digits
            return s.chars().all(|c| c.is_ascii_digit());
        }
        if self.date_format == "ss" {
            return s.chars().all(|c| c.is_ascii_digit());
        }

        // Check for YYYY-MM-DD pattern
        if s.len() >= 8 && s.contains('-') {
            return true;
        }

        // Check for YYYYMMDD pattern
        if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }

        false
    }

    /// Check if a string looks like a duration
    fn looks_like_duration(&self, s: &str) -> bool {
        let s = s.trim();
        if s.is_empty() {
            return false;
        }

        // Check for duration patterns: 1d, 2w, 3h, 4m, 5s, 6ms
        for suffix in ["ms", "w", "d", "h", "m", "s"] {
            if let Some(num_part) = s.strip_suffix(suffix) {
                if num_part.parse::<f64>().is_ok() {
                    return true;
                }
            }
        }

        false
    }
}

/// Parsed task data from the raw string
#[derive(Debug, Default)]
struct ParsedTaskData {
    id: Option<String>,
    flags: TaskFlags,
    after: Vec<String>,
    until: Vec<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    duration: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, NaiveDate};

    fn date(year: i32, month: u32, day: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn datetime(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, min, sec)
            .unwrap()
    }

    // ==================
    // Duration parsing tests
    // ==================

    #[test]
    fn test_parse_duration_1d() {
        let db = GanttDb::new();
        let (value, unit) = db.parse_duration("1d");
        assert_eq!(value, 1.0);
        assert_eq!(unit, "d");
    }

    #[test]
    fn test_parse_duration_2w() {
        let db = GanttDb::new();
        let (value, unit) = db.parse_duration("2w");
        assert_eq!(value, 2.0);
        assert_eq!(unit, "w");
    }

    #[test]
    fn test_parse_duration_1ms() {
        let db = GanttDb::new();
        let (value, unit) = db.parse_duration("1ms");
        assert_eq!(value, 1.0);
        assert_eq!(unit, "ms");
    }

    #[test]
    fn test_parse_duration_0_1s() {
        let db = GanttDb::new();
        let (value, unit) = db.parse_duration("0.1s");
        assert!((value - 0.1).abs() < f64::EPSILON);
        assert_eq!(unit, "s");
    }

    #[test]
    fn test_parse_duration_invalid_unit() {
        let db = GanttDb::new();
        let (value, unit) = db.parse_duration("1f");
        assert!(value.is_nan());
        assert_eq!(unit, "ms");
    }

    // ==================
    // Clear function tests
    // ==================

    #[test]
    fn test_clear_resets_tasks() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("test section");
        db.add_task("test1", "id1,2019-02-01,1d");
        db.clear();
        assert!(db.get_tasks().is_empty());
    }

    #[test]
    fn test_clear_resets_acc_title() {
        let mut db = GanttDb::new();
        db.set_acc_title("Test Title");
        db.clear();
        assert_eq!(db.get_acc_title(), "");
    }

    #[test]
    fn test_clear_resets_acc_description() {
        let mut db = GanttDb::new();
        db.set_acc_description("Test Description");
        db.clear();
        assert_eq!(db.get_acc_description(), "");
    }

    #[test]
    fn test_clear_resets_date_format() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.clear();
        assert_eq!(db.get_date_format(), "");
    }

    #[test]
    fn test_clear_resets_axis_format() {
        let mut db = GanttDb::new();
        db.set_axis_format("%Y-%m-%d");
        db.clear();
        assert_eq!(db.get_axis_format(), "");
    }

    #[test]
    fn test_clear_resets_today_marker() {
        let mut db = GanttDb::new();
        db.set_today_marker("off");
        db.clear();
        assert_eq!(db.get_today_marker(), "");
    }

    #[test]
    fn test_clear_resets_excludes() {
        let mut db = GanttDb::new();
        db.set_excludes("weekends 2019-02-06");
        db.clear();
        assert!(db.get_excludes().is_empty());
    }

    #[test]
    fn test_clear_resets_sections() {
        let mut db = GanttDb::new();
        db.add_section("test section");
        db.clear();
        assert!(db.get_sections().is_empty());
    }

    #[test]
    fn test_clear_resets_inclusive_end_dates() {
        let mut db = GanttDb::new();
        db.enable_inclusive_end_dates();
        db.clear();
        assert!(!db.end_dates_are_inclusive());
    }

    #[test]
    fn test_clear_resets_display_mode() {
        let mut db = GanttDb::new();
        db.set_display_mode("compact");
        db.clear();
        assert_eq!(db.get_display_mode(), "");
    }

    // ==================
    // Task parsing tests - fixed dates
    // ==================

    #[test]
    fn test_handle_fixed_dates() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("testa1");
        db.add_task("test1", "id1,2013-01-01,2013-01-12");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time, Some(date(2013, 1, 1)));
        assert_eq!(tasks[0].end_time, Some(date(2013, 1, 12)));
        assert_eq!(tasks[0].id, "id1");
        assert_eq!(tasks[0].task, "test1");
    }

    #[test]
    fn test_handle_duration_days() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("testa1");
        db.add_task("test1", "id1,2013-01-01,2d");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time, Some(date(2013, 1, 1)));
        assert_eq!(tasks[0].end_time, Some(date(2013, 1, 3)));
        assert_eq!(tasks[0].id, "id1");
        assert_eq!(tasks[0].task, "test1");
    }

    #[test]
    fn test_handle_duration_hours() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("testa1");
        db.add_task("test1", "id1,2013-01-01,2h");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time, Some(date(2013, 1, 1)));
        assert_eq!(tasks[0].end_time, Some(datetime(2013, 1, 1, 2, 0, 0)));
    }

    #[test]
    fn test_handle_duration_minutes() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("testa1");
        db.add_task("test1", "id1,2013-01-01,2m");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time, Some(date(2013, 1, 1)));
        assert_eq!(tasks[0].end_time, Some(datetime(2013, 1, 1, 0, 2, 0)));
    }

    #[test]
    fn test_handle_duration_seconds() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("testa1");
        db.add_task("test1", "id1,2013-01-01,2s");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time, Some(date(2013, 1, 1)));
        assert_eq!(tasks[0].end_time, Some(datetime(2013, 1, 1, 0, 0, 2)));
    }

    #[test]
    fn test_handle_duration_weeks() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("testa1");
        db.add_task("test1", "id1,2013-01-01,2w");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time, Some(date(2013, 1, 1)));
        assert_eq!(tasks[0].end_time, Some(date(2013, 1, 15)));
    }

    #[test]
    fn test_handle_fixed_dates_without_id() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("testa1");
        db.add_task("test1", "2013-01-01,2013-01-12");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time, Some(date(2013, 1, 1)));
        assert_eq!(tasks[0].end_time, Some(date(2013, 1, 12)));
        assert_eq!(tasks[0].id, "task1");
        assert_eq!(tasks[0].task, "test1");
    }

    #[test]
    fn test_handle_duration_without_id() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("testa1");
        db.add_task("test1", "2013-01-01,4d");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time, Some(date(2013, 1, 1)));
        assert_eq!(tasks[0].end_time, Some(date(2013, 1, 5)));
        assert_eq!(tasks[0].id, "task1");
    }

    // ==================
    // Relative date tests (after)
    // ==================

    #[test]
    fn test_relative_start_after_id() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("testa1");
        db.add_task("test1", "id1,2013-01-01,2w");
        db.add_task("test2", "id2,after id1,1d");
        let tasks = db.get_tasks();

        assert_eq!(tasks[1].start_time, Some(date(2013, 1, 15)));
        assert_eq!(tasks[1].id, "id2");
        assert_eq!(tasks[1].task, "test2");
    }

    #[test]
    fn test_relative_start_after_without_id() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("testa1");
        db.add_task("test1", "id1,2013-01-01,2w");
        db.add_task("test2", "after id1,1d");
        let tasks = db.get_tasks();

        assert_eq!(tasks[1].start_time, Some(date(2013, 1, 15)));
        assert_eq!(tasks[1].id, "task1");
    }

    #[test]
    fn test_relative_start_across_sections() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("sec1");
        db.add_task("test1", "id1,2013-01-01,2w");
        db.add_task("test2", "id2,after id3,1d");
        db.add_section("sec2");
        db.add_task("test3", "id3,after id1,2d");
        let tasks = db.get_tasks();

        assert_eq!(tasks[1].start_time, Some(date(2013, 1, 17)));
        assert_eq!(tasks[1].end_time, Some(date(2013, 1, 18)));
        assert_eq!(tasks[1].id, "id2");

        assert_eq!(tasks[2].start_time, Some(date(2013, 1, 15)));
        assert_eq!(tasks[2].end_time, Some(date(2013, 1, 17)));
        assert_eq!(tasks[2].id, "id3");
    }

    // ==================
    // Relative end date tests (until)
    // ==================

    #[test]
    fn test_relative_end_until_id() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("sec1");
        db.add_task("task1", "id1,2013-01-01,until id3");
        db.add_section("sec2");
        db.add_task("task2", "id2,2013-01-10,until id3");
        db.add_task("task3", "id3,2013-02-01,2d");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time, Some(date(2013, 1, 1)));
        assert_eq!(tasks[0].end_time, Some(date(2013, 2, 1)));
        assert_eq!(tasks[0].id, "id1");

        assert_eq!(tasks[1].start_time, Some(date(2013, 1, 10)));
        assert_eq!(tasks[1].end_time, Some(date(2013, 2, 1)));
        assert_eq!(tasks[1].id, "id2");
    }

    #[test]
    fn test_relative_start_multiple_ids() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("sec1");
        db.add_task("task1", "id1,after id2 id3 id4,1d");
        db.add_task("task2", "id2,2013-01-01,1d");
        db.add_task("task3", "id3,2013-02-01,3d");
        db.add_task("task4", "id4,2013-02-01,2d");
        let tasks = db.get_tasks();

        // Should start after the latest of id2, id3, id4
        // id3 ends on 2013-02-04 (Feb 1 + 3 days)
        assert_eq!(tasks[0].end_time, Some(date(2013, 2, 5)));
        assert_eq!(tasks[0].id, "id1");
    }

    #[test]
    fn test_relative_end_multiple_ids() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("sec1");
        db.add_task("task1", "id1,2013-01-01,until id2 id3 id4");
        db.add_task("task2", "id2,2013-01-11,1d");
        db.add_task("task3", "id3,2013-02-10,1d");
        db.add_task("task4", "id4,2013-02-12,1d");
        let tasks = db.get_tasks();

        // Should end at the earliest of id2, id3, id4 start times
        assert_eq!(tasks[0].end_time, Some(date(2013, 1, 11)));
        assert_eq!(tasks[0].id, "id1");
    }

    // ==================
    // Millisecond handling
    // ==================

    #[test]
    fn test_handle_milliseconds() {
        let mut db = GanttDb::new();
        db.set_date_format("x");
        db.add_section("testa1");
        db.add_task("test1", "id1,0,20ms");
        db.add_task("test2", "id2,after id1,5ms");
        db.add_section("testa2");
        db.add_task("test3", "id3,20,10ms");
        db.add_task("test4", "id4,after id3,0.005s");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time.unwrap().and_utc().timestamp_millis(), 0);
        assert_eq!(tasks[0].end_time.unwrap().and_utc().timestamp_millis(), 20);
        assert_eq!(
            tasks[1].start_time.unwrap().and_utc().timestamp_millis(),
            20
        );
        assert_eq!(tasks[1].end_time.unwrap().and_utc().timestamp_millis(), 25);
        assert_eq!(
            tasks[2].start_time.unwrap().and_utc().timestamp_millis(),
            20
        );
        assert_eq!(tasks[2].end_time.unwrap().and_utc().timestamp_millis(), 30);
        assert_eq!(
            tasks[3].start_time.unwrap().and_utc().timestamp_millis(),
            30
        );
        assert_eq!(tasks[3].end_time.unwrap().and_utc().timestamp_millis(), 35);
    }

    // ==================
    // Inclusive end dates
    // ==================

    #[test]
    fn test_inclusive_end_dates() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.enable_inclusive_end_dates();
        db.add_task("test1", "id1,2019-02-01,1d");
        db.add_task("test2", "id2,2019-02-01,2019-02-03");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time, Some(date(2019, 2, 1)));
        assert_eq!(tasks[0].end_time, Some(date(2019, 2, 2)));

        assert_eq!(tasks[1].start_time, Some(date(2019, 2, 1)));
        assert_eq!(tasks[1].end_time, Some(date(2019, 2, 4))); // +1 day for inclusive
        assert!(tasks[1].manual_end_time);
    }

    // ==================
    // Today marker
    // ==================

    #[test]
    fn test_today_marker_hide() {
        let mut db = GanttDb::new();
        db.set_today_marker("off");
        assert_eq!(db.get_today_marker(), "off");
    }

    #[test]
    fn test_today_marker_style() {
        let mut db = GanttDb::new();
        db.set_today_marker("stoke:stroke-width:5px,stroke:#00f,opacity:0.5");
        assert_eq!(
            db.get_today_marker(),
            "stoke:stroke-width:5px,stroke:#00f,opacity:0.5"
        );
    }

    // ==================
    // Task flags
    // ==================

    #[test]
    fn test_task_flags_done() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("section");
        db.add_task("Completed task", "done, des1, 2014-01-06, 2014-01-08");
        let tasks = db.get_tasks();

        assert!(tasks[0].flags.done);
        assert_eq!(tasks[0].id, "des1");
    }

    #[test]
    fn test_task_flags_active() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("section");
        db.add_task("Active task", "active, des2, 2014-01-09, 3d");
        let tasks = db.get_tasks();

        assert!(tasks[0].flags.active);
        assert_eq!(tasks[0].id, "des2");
    }

    #[test]
    fn test_task_flags_crit() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("section");
        db.add_task("Critical task", "crit, done, 2014-01-06, 24h");
        let tasks = db.get_tasks();

        assert!(tasks[0].flags.critical);
        assert!(tasks[0].flags.done);
    }

    #[test]
    fn test_task_flags_vert() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("section");
        db.add_task("Sprint Start", "vert, sprint1, 2024-01-15, 1d");
        let tasks = db.get_tasks();

        assert!(tasks[0].flags.vert);
        assert_eq!(tasks[0].id, "sprint1");
        assert_eq!(tasks[0].task, "Sprint Start");
    }

    #[test]
    fn test_task_flags_milestone() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("section");
        db.add_task("Release", "milestone, rel1, 2024-02-01, 1d");
        let tasks = db.get_tasks();

        assert!(tasks[0].flags.milestone);
        assert_eq!(tasks[0].id, "rel1");
    }

    // ==================
    // Task order
    // ==================

    #[test]
    fn test_task_order_maintained() {
        let mut db = GanttDb::new();
        db.set_acc_title("Project Execution");
        db.set_date_format("YYYY-MM-DD");
        db.add_section("section A section");
        db.add_task("Completed task", "done, des1, 2014-01-06, 2014-01-08");
        db.add_task("Active task", "active, des2, 2014-01-09, 3d");
        db.add_task("Future task", "des3, after des2, 5d");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].order, 0);
        assert_eq!(tasks[0].id, "des1");

        assert_eq!(tasks[1].order, 1);
        assert_eq!(tasks[1].id, "des2");

        assert_eq!(tasks[2].order, 2);
        assert_eq!(tasks[2].id, "des3");
    }

    // ==================
    // Edge cases
    // ==================

    #[test]
    fn test_end_date_on_31st() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("Task endTime is on the 31st");
        db.add_task("test1", "id1,2019-09-30,11d");
        db.add_task("test2", "id2,after id1,20d");
        let tasks = db.get_tasks();

        assert_eq!(tasks[0].start_time, Some(date(2019, 9, 30)));
        assert_eq!(tasks[0].end_time, Some(date(2019, 10, 11)));

        assert_eq!(tasks[1].start_time, Some(date(2019, 10, 11)));
        assert_eq!(tasks[1].end_time, Some(date(2019, 10, 31)));
    }

    #[test]
    fn test_seconds_only_format() {
        let mut db = GanttDb::new();
        db.set_date_format("ss");
        db.add_section("Network Request");
        db.add_task("RTT", "rtt, 0, 20");
        let tasks = db.get_tasks();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task, "RTT");
        assert_eq!(tasks[0].id, "rtt");
    }

    #[test]
    fn test_year_typo_202_instead_of_2024() {
        let mut db = GanttDb::new();
        db.set_date_format("YYYY-MM-DD");
        db.add_section("Vacation");
        db.add_task("London Trip 1", "2024-12-01, 7d");
        db.add_task("London Trip 2", "202-12-01, 7d");
        let tasks = db.get_tasks();

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].start_time.unwrap().year(), 2024);
        // Second task with year 202 - chrono will parse what it can
        // or fail gracefully
    }

    // ==================
    // Weekend handling
    // ==================

    #[test]
    fn test_set_weekend_friday() {
        let mut db = GanttDb::new();
        db.set_weekend("friday");
        assert_eq!(db.weekend_start, WeekendStart::Friday);
    }

    #[test]
    fn test_set_weekend_saturday_default() {
        let mut db = GanttDb::new();
        db.set_weekend("saturday");
        assert_eq!(db.weekend_start, WeekendStart::Saturday);
    }
}

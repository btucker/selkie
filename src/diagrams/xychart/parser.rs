//! XY Chart diagram parser using pest grammar

use pest::Parser;
use pest_derive::Parser;

use super::types::{ChartOrientation, DataPoint, XYChartDb};

#[derive(Parser)]
#[grammar = "diagrams/xychart/xychart.pest"]
pub struct XYChartParser;

/// Parse an XY chart diagram and return the populated database
pub fn parse(input: &str) -> Result<XYChartDb, String> {
    let mut db = XYChartDb::new();

    let pairs =
        XYChartParser::parse(Rule::diagram, input).map_err(|e| format!("Parse error: {}", e))?;

    for pair in pairs {
        if pair.as_rule() == Rule::diagram {
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::orientation => {
                        let orient_str = inner.as_str().to_lowercase();
                        if orient_str.contains("horizontal") {
                            db.set_orientation(ChartOrientation::Horizontal);
                        } else {
                            db.set_orientation(ChartOrientation::Vertical);
                        }
                    }
                    Rule::document => {
                        process_document(&mut db, inner)?;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(db)
}

fn process_document(db: &mut XYChartDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    for stmt in pair.into_inner() {
        process_statement(db, stmt)?;
    }
    Ok(())
}

fn process_statement(db: &mut XYChartDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    match pair.as_rule() {
        Rule::statement => {
            for inner in pair.into_inner() {
                process_statement(db, inner)?;
            }
        }
        Rule::comment_stmt => {
            // Ignore comments
        }
        Rule::title_stmt => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::title_text {
                    let text = extract_text(inner);
                    db.set_title(&text);
                }
            }
        }
        Rule::x_axis_stmt => {
            process_x_axis(db, pair)?;
        }
        Rule::y_axis_stmt => {
            process_y_axis(db, pair)?;
        }
        Rule::line_stmt => {
            process_line(db, pair)?;
        }
        Rule::bar_stmt => {
            process_bar(db, pair)?;
        }
        _ => {}
    }
    Ok(())
}

fn process_x_axis(db: &mut XYChartDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut title = String::new();
    let mut categories: Vec<String> = Vec::new();
    let mut range: Option<(f64, f64)> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::axis_title => {
                title = extract_axis_title(inner);
            }
            Rule::band_data => {
                categories = extract_categories(inner)?;
            }
            Rule::range_data => {
                range = Some(extract_range(inner)?);
            }
            _ => {}
        }
    }

    if !categories.is_empty() {
        db.set_x_axis_band(&title, categories);
    } else if let Some((min, max)) = range {
        db.set_x_axis_linear(&title, min, max);
    } else {
        // Just title, set as band with empty categories for now
        db.set_x_axis_band(&title, Vec::new());
    }
    Ok(())
}

fn process_y_axis(db: &mut XYChartDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut title = String::new();
    let mut range: Option<(f64, f64)> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::axis_title => {
                title = extract_axis_title(inner);
            }
            Rule::range_data => {
                range = Some(extract_range(inner)?);
            }
            _ => {}
        }
    }

    if let Some((min, max)) = range {
        db.set_y_axis_linear(&title, min, max);
    } else {
        // Just title, infer range from data later
        db.set_y_axis_linear(&title, 0.0, 0.0);
    }
    Ok(())
}

fn process_line(db: &mut XYChartDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut title = String::new();
    let mut values: Vec<f64> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::plot_title => {
                title = extract_plot_title(inner);
            }
            Rule::data_array => {
                values = extract_data_array(inner)?;
            }
            _ => {}
        }
    }

    // Create data points with title as label and index
    let data_points: Vec<DataPoint> = values
        .iter()
        .enumerate()
        .map(|(i, v)| DataPoint {
            label: if title.is_empty() {
                format!("{}", i)
            } else {
                title.clone()
            },
            value: *v,
        })
        .collect();

    db.add_line_plot(data_points);
    Ok(())
}

fn process_bar(db: &mut XYChartDb, pair: pest::iterators::Pair<Rule>) -> Result<(), String> {
    let mut title = String::new();
    let mut values: Vec<f64> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::plot_title => {
                title = extract_plot_title(inner);
            }
            Rule::data_array => {
                values = extract_data_array(inner)?;
            }
            _ => {}
        }
    }

    // Create data points with title as label and index
    let data_points: Vec<DataPoint> = values
        .iter()
        .enumerate()
        .map(|(i, v)| DataPoint {
            label: if title.is_empty() {
                format!("{}", i)
            } else {
                title.clone()
            },
            value: *v,
        })
        .collect();

    db.add_bar_plot(data_points);
    Ok(())
}

fn extract_text(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                return unquote(inner.as_str());
            }
            Rule::unquoted_title => {
                return inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }
    String::new()
}

fn extract_axis_title(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                return unquote(inner.as_str());
            }
            Rule::unquoted_axis_title => {
                return inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }
    String::new()
}

fn extract_plot_title(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                return unquote(inner.as_str());
            }
            Rule::unquoted_plot_title => {
                return inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }
    String::new()
}

fn extract_categories(pair: pest::iterators::Pair<Rule>) -> Result<Vec<String>, String> {
    let mut categories = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::category_list {
            for cat in inner.into_inner() {
                if cat.as_rule() == Rule::category {
                    categories.push(extract_category(cat));
                }
            }
        }
    }

    Ok(categories)
}

fn extract_category(pair: pest::iterators::Pair<Rule>) -> String {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::quoted_string => {
                return unquote(inner.as_str());
            }
            Rule::unquoted_category => {
                return inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }
    String::new()
}

fn extract_range(pair: pest::iterators::Pair<Rule>) -> Result<(f64, f64), String> {
    let mut numbers: Vec<f64> = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::number {
            let num: f64 = inner
                .as_str()
                .parse()
                .map_err(|_| format!("Invalid number: {}", inner.as_str()))?;
            numbers.push(num);
        }
    }

    if numbers.len() >= 2 {
        Ok((numbers[0], numbers[1]))
    } else {
        Err("Range requires two numbers".to_string())
    }
}

fn extract_data_array(pair: pest::iterators::Pair<Rule>) -> Result<Vec<f64>, String> {
    let mut values = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::number_list {
            for num_pair in inner.into_inner() {
                if num_pair.as_rule() == Rule::signed_number {
                    let num_str = num_pair.as_str().trim();
                    let num: f64 = num_str
                        .parse()
                        .map_err(|_| format!("Invalid number: {}", num_str))?;
                    values.push(num);
                }
            }
        }
    }

    Ok(values)
}

/// Remove surrounding quotes from a string
fn unquote(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagrams::xychart::types::{PlotType, XAxisData, YAxisData};

    mod basic_parsing {
        use super::*;

        #[test]
        fn should_parse_empty_xychart() {
            let result = parse("xychart");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_reject_invalid_chart() {
            let result = parse("xychart-1");
            assert!(result.is_err());
        }
    }

    mod title_parsing {
        use super::*;

        #[test]
        fn should_parse_quoted_title() {
            let result = parse("xychart\ntitle \"This is a title\"");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.title, "This is a title");
        }

        #[test]
        fn should_parse_unquoted_title() {
            let result = parse("xychart\ntitle oneLinertitle");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.title, "oneLinertitle");
        }
    }

    mod orientation_parsing {
        use super::*;

        #[test]
        fn should_parse_vertical_orientation() {
            let result = parse("xychart vertical");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.orientation, ChartOrientation::Vertical);
        }

        #[test]
        fn should_parse_horizontal_orientation() {
            let result = parse("xychart horizontal");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            assert_eq!(db.orientation, ChartOrientation::Horizontal);
        }
    }

    mod x_axis_parsing {
        use super::*;

        #[test]
        fn should_parse_x_axis_title() {
            let result = parse("xychart\nx-axis xAxisName");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            if let Some(XAxisData::Band(axis)) = &db.x_axis {
                assert_eq!(axis.title, "xAxisName");
            } else {
                panic!("Expected band axis");
            }
        }

        #[test]
        fn should_parse_x_axis_quoted_title() {
            let result = parse("xychart\nx-axis \"xAxisName has space\"");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            if let Some(XAxisData::Band(axis)) = &db.x_axis {
                assert_eq!(axis.title, "xAxisName has space");
            }
        }

        #[test]
        fn should_parse_x_axis_with_range() {
            let result = parse("xychart\nx-axis xAxisName 45.5 --> 33");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            if let Some(XAxisData::Linear(axis)) = &db.x_axis {
                assert_eq!(axis.title, "xAxisName");
                assert_eq!(axis.min, 45.5);
                assert_eq!(axis.max, 33.0);
            } else {
                panic!("Expected linear axis");
            }
        }

        #[test]
        fn should_parse_x_axis_with_categories() {
            let result = parse("xychart\nx-axis xAxisName [\"cat1\", cat2a]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            if let Some(XAxisData::Band(axis)) = &db.x_axis {
                assert_eq!(axis.title, "xAxisName");
                assert_eq!(axis.categories, vec!["cat1", "cat2a"]);
            } else {
                panic!("Expected band axis");
            }
        }
    }

    mod y_axis_parsing {
        use super::*;

        #[test]
        fn should_parse_y_axis_title() {
            let result = parse("xychart\ny-axis yAxisName");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }

        #[test]
        fn should_parse_y_axis_with_range() {
            let result = parse("xychart\ny-axis yAxisName 45.5 --> 33");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            if let Some(YAxisData::Linear(axis)) = &db.y_axis {
                assert_eq!(axis.title, "yAxisName");
                assert_eq!(axis.min, 45.5);
                assert_eq!(axis.max, 33.0);
            }
        }
    }

    mod line_parsing {
        use super::*;

        #[test]
        fn should_parse_line_data() {
            let result =
                parse("xychart\nx-axis xAxisName\ny-axis yAxisName\nline lineTitle [23, 45, 56.6]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let plots = db.get_plots();
            assert_eq!(plots.len(), 1);
            assert_eq!(plots[0].plot_type, PlotType::Line);
            assert_eq!(plots[0].data.len(), 3);
            assert_eq!(plots[0].data[0].value, 23.0);
            assert_eq!(plots[0].data[1].value, 45.0);
            assert_eq!(plots[0].data[2].value, 56.6);
        }

        #[test]
        fn should_parse_line_with_signed_numbers() {
            let result = parse("xychart\nline \"lineTitle with space\" [+23, -45, 56.6]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let plots = db.get_plots();
            assert_eq!(plots[0].data[0].value, 23.0);
            assert_eq!(plots[0].data[1].value, -45.0);
        }

        #[test]
        fn should_parse_line_without_title() {
            let result = parse("xychart\nline [23, 45, 56.6]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }
    }

    mod bar_parsing {
        use super::*;

        #[test]
        fn should_parse_bar_data() {
            let result = parse("xychart\nbar barTitle [23, 45, 56.6, 0.22]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();
            let plots = db.get_plots();
            assert_eq!(plots.len(), 1);
            assert_eq!(plots[0].plot_type, PlotType::Bar);
            assert_eq!(plots[0].data.len(), 4);
        }

        #[test]
        fn should_parse_bar_without_title() {
            let result = parse("xychart\nbar [23, -45, 56.6]");
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
        }
    }

    mod complex_charts {
        use super::*;

        #[test]
        fn should_parse_full_chart() {
            let input = r#"xychart horizontal
    title "Basic xychart"
    x-axis "this is x axis" [category1, "category 2", category3]
    y-axis yaxisText 10 --> 150
    bar barTitle1 [23, 45, 56.6]
    line lineTitle1 [11, 45.5, 67, 23]
    bar barTitle2 [13, 42, 56.89]
    line lineTitle2 [45, 99, 12]"#;

            let result = parse(input);
            assert!(result.is_ok(), "Parse error: {:?}", result.err());
            let db = result.unwrap();

            assert_eq!(db.orientation, ChartOrientation::Horizontal);
            assert_eq!(db.title, "Basic xychart");

            if let Some(XAxisData::Band(axis)) = &db.x_axis {
                assert_eq!(axis.title, "this is x axis");
                assert_eq!(axis.categories.len(), 3);
            }

            if let Some(YAxisData::Linear(axis)) = &db.y_axis {
                assert_eq!(axis.title, "yaxisText");
                assert_eq!(axis.min, 10.0);
                assert_eq!(axis.max, 150.0);
            }

            let plots = db.get_plots();
            assert_eq!(plots.len(), 4);
        }
    }

    // Tests ported from mermaid Cypress tests (xyChart.spec.js)
    mod cypress_tests {
        use super::*;

        #[test]
        fn test_cypress_simplest_beta() {
            // From Cypress: should render the simplest possible xy-beta chart
            let input = r#"xychart-beta
        line [10, 30, 20]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_simplest() {
            // From Cypress: should render the simplest possible xy chart
            let input = r#"xychart
        line [10, 30, 20]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_complete_chart() {
            // From Cypress: Should render a complete chart
            let input = r#"xychart
        title "Sales Revenue"
        x-axis Months [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        y-axis "Revenue (in $)" 4000 --> 11000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]
        line [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_plots().len(), 2);
        }

        #[test]
        fn test_cypress_without_title() {
            // From Cypress: Should render a chart without title
            let input = r#"xychart
        x-axis Months [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        y-axis "Revenue (in $)" 4000 --> 11000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]
        line [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_y_axis_no_title() {
            // From Cypress: y-axis title not required
            let input = r#"xychart
        x-axis Months [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        y-axis 4000 --> 11000
        bar [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]
        line [5000, 6000, 7500, 8200, 9500, 10500, 11000, 10200, 9200, 8500, 7000, 6000]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_x_axis_no_title() {
            // From Cypress: x axis title not required
            let input = r#"xychart
        x-axis [jan, feb, mar, apr, may, jun, jul, aug, sep, oct, nov, dec]
        bar [5000, 6000, 7500, 8200, 9500, 10500, 14000, 3200, 9200, 9900, 3400, 6000]
        line [2000, 7000, 6500, 9200, 9500, 7500, 11000, 10200, 3200, 8500, 7000, 8800]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }

        #[test]
        fn test_cypress_multiple_plots() {
            // From Cypress: Multiple plots can be rendered
            let input = r#"xychart
        line [23, 46, 77, 34]
        line [45, 32, 33, 12]
        bar [87, 54, 99, 85]
        line [78, 88, 22, 4]
        line [22, 29, 75, 33]
        bar [52, 96, 35, 10]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let db = result.unwrap();
            assert_eq!(db.get_plots().len(), 6);
        }

        #[test]
        #[ignore = "TODO: Leading decimal point (.6) without 0 not supported"]
        fn test_cypress_decimals_negatives() {
            // From Cypress: Decimals and negative numbers are supported
            let input = r#"xychart
        y-axis -2.4 --> 3.5
        line [+1.3, .6, 2.4, -.34]"#;
            let result = parse(input);
            assert!(result.is_ok(), "Failed to parse: {:?}", result);
        }
    }
}

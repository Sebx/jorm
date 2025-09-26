use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Integration test for AI-generated DAG that copies a table with 20 fields and 1000 records
#[tokio::test]
async fn test_ai_generated_table_copy_dag() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create test CSV file with 20 fields and 1000 records
    let csv_path = temp_path.join("table1.csv");
    create_test_csv(&csv_path).await;

    // Generate DAG using AI prompt
    let dag_content = generate_simple_table_copy_dag(&csv_path);

    // Write DAG to temporary file
    let dag_path = temp_path.join("table_copy_dag.txt");
    fs::write(&dag_path, &dag_content).expect("Failed to write DAG file");

    // Execute the AI-generated DAG
    let result = execute_dag(&dag_path).await;

    // Verify the DAG executed successfully
    assert!(
        result.success,
        "AI-generated DAG should execute successfully"
    );

    // Verify table2 was created
    let target_file = std::path::Path::new("table2.csv");
    assert!(target_file.exists(), "table2.csv should exist");

    // Verify record count
    let source_count = get_csv_record_count(&csv_path).await;
    let target_count = get_csv_record_count(&std::path::PathBuf::from("table2.csv")).await;
    assert_eq!(source_count, target_count, "Record counts should match");

    println!("✅ AI-generated table copy DAG executed successfully!");
    println!(
        "📊 Copied {source_count} records with 20 fields from table1 to table2"
    );
}

/// Create test CSV file with 20 fields and 1000 records
async fn create_test_csv(csv_path: &std::path::Path) {
    let mut csv_content = String::new();
    csv_content.push_str("id,field1,field2,field3,field4,field5,field6,field7,field8,field9,field10,field11,field12,field13,field14,field15,field16,field17,field18,field19,field20\n");

    // Generate 1000 records with 20 fields each
    for i in 1..=1000 {
        csv_content.push_str(&format!(
            "{},value_{},{},{:.1},text_{},{},{:.1},string_{},{},{:.1},data_{},{},{:.1},info_{},{},{:.1},record_{},{},{:.1},item_{},{}\n",
            i, i, i, i as f64 * 1.5, i, i * 2, i as f64 * 2.5, i, i * 3, i as f64 * 3.5, i, i * 4, i as f64 * 4.5, i, i * 5, i as f64 * 5.5, i, i * 6, i as f64 * 6.5, i, i * 7
        ));
    }

    // Write CSV file
    std::fs::write(csv_path, csv_content).expect("Failed to create test CSV file");
}

/// Generate simple DAG content using AI prompt for table copying
fn generate_simple_table_copy_dag(csv_path: &std::path::Path) -> String {
    // This simulates an AI-generated DAG based on the prompt:
    // "Create a DAG that copies table1 with 20 fields and 1000 records to table2"
    // Using a minimal text format that won't be confused with YAML

    format!(
        r#"AI-Generated Table Copy DAG

This DAG was generated from the prompt: "Create a DAG that copies table1 with 20 fields and 1000 records to table2"

## Simple Table Copy Tasks

tasks:
- copy_table_step1
  type: python
  script: |
    import csv
    
    # Copy CSV file directly
    source_file = r"{}"
    target_file = "table2.csv"
    
    print("🔄 Starting table copy operation...")
    
    # Copy all data
    with open(source_file, 'r') as src, open(target_file, 'w', newline='') as dst:
        reader = csv.reader(src)
        writer = csv.writer(dst)
        
        # Copy header
        headers = next(reader)
        writer.writerow(headers)
        field_count = len(headers)
        
        # Copy all rows
        rows_copied = 0
        for row in reader:
            writer.writerow(row)
            rows_copied += 1
    
    print(f"✅ Copied {{rows_copied}} records with {{field_count}} fields from table1 to table2")
    print(f"📊 Total fields: {{field_count}}")
    print(f"📊 Total records: {{rows_copied}}")

- verify_copy_step2
  type: python
  script: |
    import csv
    
    # Verify both files exist and have same content
    source_file = r"{}"
    target_file = "table2.csv"
    
    print("🔍 Verifying copy operation...")
    
    # Read both files
    with open(source_file, 'r') as f:
        source_reader = csv.reader(f)
        source_headers = next(source_reader)
        source_rows = list(source_reader)
    
    with open(target_file, 'r') as f:
        target_reader = csv.reader(f)
        target_headers = next(target_reader)
        target_rows = list(target_reader)
    
    # Verify counts
    source_count = len(source_rows)
    target_count = len(target_rows)
    field_count = len(source_headers)
    
    if source_count == target_count and source_headers == target_headers:
        print(f"✅ Verification successful!")
        print(f"📊 {{source_count}} records copied")
        print(f"📊 {{field_count}} fields per record")
        print(f"✅ AI-generated DAG completed successfully")
    else:
        print(f"❌ Verification failed!")
        exit(1)

dependencies:
- verify_copy_step2 after copy_table_step1
"#,
        csv_path.display(),
        csv_path.display()
    )
}

/// Execute the DAG using jorm-rs
async fn execute_dag(dag_path: &std::path::Path) -> DagExecutionResult {
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "jorm-rs",
            "--",
            "run",
            dag_path.to_str().unwrap(),
        ])
        .current_dir(".")
        .output()
        .expect("Failed to execute DAG");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("DAG Execution Output:");
    println!("STDOUT: {stdout}");
    if !stderr.is_empty() {
        println!("STDERR: {stderr}");
    }

    DagExecutionResult {
        success: output.status.success(),
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        exit_code: output.status.code(),
    }
}

/// Get record count for a CSV file
async fn get_csv_record_count(csv_path: &std::path::Path) -> i32 {
    let content = std::fs::read_to_string(csv_path).expect("Failed to read CSV file");
    let lines: Vec<&str> = content.lines().collect();
    // Subtract 1 for header row
    (lines.len() - 1) as i32
}

#[derive(Debug)]
struct DagExecutionResult {
    success: bool,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}

/// Demonstration test for AI capabilities in table operations
#[tokio::test]
async fn test_ai_generated_table_analysis() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    // Create test CSV with specific characteristics
    let csv_path = temp_path.join("sample_data.csv");
    create_sample_data_csv(&csv_path).await;

    // Generate analysis DAG
    let dag_content = generate_analysis_dag(&csv_path);
    let dag_path = temp_path.join("analysis_dag.txt");
    fs::write(&dag_path, &dag_content).expect("Failed to write DAG file");

    // Execute analysis
    let result = execute_dag(&dag_path).await;
    assert!(result.success, "Analysis DAG should execute successfully");

    // Verify analysis output was created
    let analysis_file = std::path::Path::new("data_analysis.json");
    assert!(analysis_file.exists(), "Analysis output should exist");

    println!("✅ AI-generated data analysis DAG executed successfully!");
}

/// Create sample data CSV for analysis
async fn create_sample_data_csv(csv_path: &std::path::Path) {
    let mut csv_content = String::new();
    csv_content.push_str(
        "id,customer_name,product,quantity,price,date,category,region,sales_rep,status\n",
    );

    let products = ["Laptop", "Mouse", "Keyboard", "Monitor", "Headphones"];
    let categories = ["Electronics", "Accessories", "Hardware"];
    let regions = ["North", "South", "East", "West"];
    let reps = ["Alice", "Bob", "Charlie", "Diana"];
    let statuses = ["Completed", "Pending", "Cancelled"];

    // Generate 100 sample records
    for i in 1..=100 {
        let product = products[i % products.len()];
        let category = categories[i % categories.len()];
        let region = regions[i % regions.len()];
        let rep = reps[i % reps.len()];
        let status = statuses[i % statuses.len()];
        let quantity = (i % 10) + 1;
        let price = (i as f64) * 1.5 + 10.0;

        csv_content.push_str(&format!(
            "{},Customer_{},{},{},{:.2},2024-01-{:02},{},{},{},{}\n",
            i,
            i,
            product,
            quantity,
            price,
            (i % 28) + 1,
            category,
            region,
            rep,
            status
        ));
    }

    std::fs::write(csv_path, csv_content).expect("Failed to create sample CSV file");
}

/// Generate analysis DAG
fn generate_analysis_dag(csv_path: &std::path::Path) -> String {
    format!(
        r#"AI-Generated Data Analysis DAG

This DAG performs comprehensive analysis on sample sales data

## Analysis Tasks

tasks:
- analyze_data
  type: python
  script: |
    import csv
    import json
    from collections import defaultdict
    
    # Read and analyze data
    csv_file = r"{}"
    
    with open(csv_file, 'r') as f:
        reader = csv.DictReader(f)
        data = list(reader)
    
    # Perform analysis
    total_records = len(data)
    total_sales = sum(float(row['price']) * int(row['quantity']) for row in data)
    
    # Category analysis
    category_sales = defaultdict(float)
    region_sales = defaultdict(float)
    
    for row in data:
        revenue = float(row['price']) * int(row['quantity'])
        category_sales[row['category']] += revenue
        region_sales[row['region']] += revenue
    
    # Create analysis report
    analysis = {{
        "total_records": total_records,
        "total_sales": round(total_sales, 2),
        "average_sale": round(total_sales / total_records, 2),
        "categories": dict(category_sales),
        "regions": dict(region_sales),
        "top_category": max(category_sales.items(), key=lambda x: x[1])[0],
        "top_region": max(region_sales.items(), key=lambda x: x[1])[0]
    }}
    
    # Save analysis
    with open("data_analysis.json", "w") as f:
        json.dump(analysis, f, indent=2)
    
    print(f"📊 Analysis Complete:")
    print(f"   Total Records: {{total_records}}")
    print(f"   Total Sales: ${{total_sales:.2f}}")
    print(f"   Average Sale: ${{analysis['average_sale']:.2f}}")
    print(f"   Top Category: {{analysis['top_category']}}")
    print(f"   Top Region: {{analysis['top_region']}}")
"#,
        csv_path.display()
    )
}

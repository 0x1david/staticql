#[cfg(test)]
mod tests {
    use crate::*;
    use rustpython_parser::{
        Parse,
        ast::{self},
    };

    fn create_test_finder() -> SqlFinder {
        SqlFinder::new(FinderConfig {
            variables: vec![
                "query".to_string(),
                "sql".to_string(),
                "also_query".to_string(),
            ],
            min_sql_length: 1,
        })
    }

    fn test_find(code: &str, expected: Vec<(&str, &str)>, test_name: &str) {
        let parsed = ast::Suite::parse(code, "test.py").expect("Failed to parse");
        let finder = create_test_finder();
        let mut contexts = Vec::new();
        finder.analyze_stmts(&parsed, "test.py", &mut contexts);

        assert_eq!(
            contexts.len(),
            expected.len(),
            "{}: Expected {} SQL strings, found {}",
            test_name,
            expected.len(),
            contexts.len()
        );

        for (i, (expected_var, expected_sql)) in expected.iter().enumerate() {
            assert!(
                i < contexts.len(),
                "{}: Missing context at index {}",
                test_name,
                i
            );

            assert_eq!(
                contexts[i].variable_name, *expected_var,
                "{}: Expected variable '{}', found '{}'",
                test_name, expected_var, contexts[i].variable_name
            );

            assert_eq!(
                contexts[i].sql_content, *expected_sql,
                "{}: Expected SQL '{}', found '{}'",
                test_name, expected_sql, contexts[i].sql_content
            );
        }
    }

    #[test]
    fn test_simple_assignment() {
        test_find(
            r#"query = "SELECT id, name FROM users WHERE active = 1""#,
            vec![("query", "SELECT id, name FROM users WHERE active = 1")],
            "simple assignment",
        );
    }

    #[test]
    fn test_multiple_assignment() {
        test_find(
            r#"query = sql = "UPDATE users SET last_login = NOW()""#,
            vec![
                ("query", "UPDATE users SET last_login = NOW()"),
                ("sql", "UPDATE users SET last_login = NOW()"),
            ],
            "multiple assignment",
        );
    }

    #[test]
    fn test_chained_multiple_assignment() {
        test_find(
            r#"query = sql = query = "DELETE FROM sessions WHERE expires_at < NOW()""#,
            vec![
                ("query", "DELETE FROM sessions WHERE expires_at < NOW()"),
                ("sql", "DELETE FROM sessions WHERE expires_at < NOW()"),
                ("query", "DELETE FROM sessions WHERE expires_at < NOW()"),
            ],
            "chained multiple assignment",
        );
    }

    #[test]
    fn test_tuple_assignment() {
        test_find(
            r#"(query, sql) = ("SELECT * FROM users", "SELECT * FROM orders WHERE status = 'pending'")"#,
            vec![
                ("query", "SELECT * FROM users"),
                ("sql", "SELECT * FROM orders WHERE status = 'pending'"),
            ],
            "tuple assignment",
        );
    }

    #[test]
    fn test_list_assignment() {
        test_find(
            r#"[query, sql] = ["SELECT COUNT(*) FROM products", "INSERT INTO audit_log (action, timestamp) VALUES ('login', NOW())"]"#,
            vec![
                ("query", "SELECT COUNT(*) FROM products"),
                (
                    "sql",
                    "INSERT INTO audit_log (action, timestamp) VALUES ('login', NOW())",
                ),
            ],
            "list assignment",
        );
    }

    #[test]
    fn test_mixed_tuple_list() {
        test_find(
            r#"(query, sql) = ["SELECT * FROM cache WHERE key = ?", "UPDATE cache SET value = ?, updated_at = NOW() WHERE key = ?"]"#,
            vec![
                ("query", "SELECT * FROM cache WHERE key = ?"),
                (
                    "sql",
                    "UPDATE cache SET value = ?, updated_at = NOW() WHERE key = ?",
                ),
            ],
            "mixed tuple/list assignment",
        );
    }

    #[test]
    fn test_nested_tuple_assignment() {
        test_find(
            r#"((query, sql), query) = (("SELECT u.* FROM users u", "SELECT r.* FROM roles r"), "SELECT * FROM admins WHERE permissions LIKE '%super%'")"#,
            vec![
                ("query", "SELECT u.* FROM users u"),
                ("sql", "SELECT r.* FROM roles r"),
                (
                    "query",
                    "SELECT * FROM admins WHERE permissions LIKE '%super%'",
                ),
            ],
            "nested tuple assignment",
        );
    }

    #[test]
    fn test_deep_nested_assignment() {
        test_find(
            r#"(((query, sql), query), sql) = ((("SELECT 1", "SELECT 2"), "SELECT 3"), "SELECT 4")"#,
            vec![
                ("query", "SELECT 1"),
                ("sql", "SELECT 2"),
                ("query", "SELECT 3"),
                ("sql", "SELECT 4"),
            ],
            "deep nested assignment",
        );
    }

    #[test]
    fn test_attribute_assignment() {
        test_find(
            r#"database.query = "SELECT u.id, u.email, p.name FROM users u JOIN profiles p ON u.id = p.user_id""#,
            vec![(
                "query",
                "SELECT u.id, u.email, p.name FROM users u JOIN profiles p ON u.id = p.user_id",
            )],
            "attribute assignment",
        );
    }

    #[test]
    fn test_class_attribute_assignment() {
        test_find(
            r#"UserModel.sql = "SELECT id, created_at, updated_at FROM users""#,
            vec![("sql", "SELECT id, created_at, updated_at FROM users")],
            "class attribute assignment",
        );
    }

    #[test]
    fn test_nested_attribute_assignment() {
        test_find(
            r#"app.db.queries.sql = "SELECT * FROM users WHERE deleted_at IS NULL""#,
            vec![("sql", "SELECT * FROM users WHERE deleted_at IS NULL")],
            "nested attribute assignment",
        );
    }

    #[test]
    fn test_subscript_assignment() {
        test_find(
            r#"queries["query"] = "SELECT * FROM users WHERE username = ? OR email = ?""#,
            vec![], // Subscripts not currently handled
            "subscript assignment",
        );
    }

    #[test]
    fn test_starred_assignment_beginning() {
        test_find(
            r#"*rest, query = ["SELECT 1", "SELECT 2", "SELECT * FROM users ORDER BY created_at DESC"]"#,
            vec![("query", "SELECT * FROM users ORDER BY created_at DESC")],
            "starred assignment at beginning",
        );
    }

    #[test]
    fn test_starred_assignment_middle() {
        test_find(
            r#"query, *middle, sql = ["SELECT 1", "SELECT 2", "SELECT 3", "SELECT * FROM orders"]"#,
            vec![("query", "SELECT 1"), ("sql", "SELECT * FROM orders")],
            "starred assignment in middle",
        );
    }

    #[test]
    fn test_starred_assignment_end() {
        test_find(
            r#"query, *rest = ["SELECT u.*, COUNT(o.id) as order_count FROM users u LEFT JOIN orders o ON u.id = o.user_id GROUP BY u.id", "SELECT 1", "SELECT 2"]"#,
            vec![(
                "query",
                "SELECT u.*, COUNT(o.id) as order_count FROM users u LEFT JOIN orders o ON u.id = o.user_id GROUP BY u.id",
            )],
            "starred assignment at end",
        );
    }

    #[test]
    fn test_mixed_names_and_attributes() {
        test_find(
            r#"query, obj.sql = ("SELECT * FROM local_users", "SELECT * FROM remote_users WHERE sync_status = 'pending'")"#,
            vec![
                ("query", "SELECT * FROM local_users"),
                (
                    "sql",
                    "SELECT * FROM remote_users WHERE sync_status = 'pending'",
                ),
            ],
            "mixed names and attributes",
        );
    }

    #[test]
    fn test_mixed_starred_and_regular() {
        test_find(
            r#"query, *middle, sql = ("SELECT * FROM primary_table", "SELECT * FROM secondary1", "SELECT * FROM secondary2", "SELECT * FROM fallback_table")"#,
            vec![
                ("query", "SELECT * FROM primary_table"),
                ("sql", "SELECT * FROM fallback_table"),
            ],
            "mixed starred and regular",
        );
    }

    #[test]
    fn test_multiline_string_assignment() {
        let expected_sql = r#"
            SELECT 
                u.id,
                u.username,
                u.email,
                COUNT(o.id) as order_count,
                SUM(o.total) as total_spent
            FROM users u
            LEFT JOIN orders o ON u.id = o.user_id
            WHERE u.created_at >= '2023-01-01'
            GROUP BY u.id, u.username, u.email
            HAVING COUNT(o.id) > 0
            ORDER BY total_spent DESC
            LIMIT 100
            "#;

        test_find(
            &format!(r#"query = """{}""""#, expected_sql),
            vec![("query", expected_sql)],
            "multiline string assignment",
        );
    }

    #[test]
    fn test_single_quoted_sql() {
        test_find(
            r#"sql = 'SELECT * FROM products WHERE category = "electronics" AND price > 100'"#,
            vec![(
                "sql",
                r#"SELECT * FROM products WHERE category = "electronics" AND price > 100"#,
            )],
            "single quoted SQL",
        );
    }

    #[test]
    fn test_raw_string_with_escapes() {
        test_find(
            r#"query = r"SELECT * FROM logs WHERE message REGEXP '^Error.*\d{4}-\d{2}-\d{2}'""#,
            vec![(
                "query",
                r#"SELECT * FROM logs WHERE message REGEXP '^Error.*\d{4}-\d{2}-\d{2}'"#,
            )],
            "raw string with regex",
        );
    }

    #[test]
    fn test_sql_with_comments() {
        let expected_sql = r#"
-- Get active users with their order counts
SELECT 
    u.id,
    u.username,
    COUNT(o.id) as order_count
FROM users u  -- Main users table
LEFT JOIN orders o ON u.id = o.user_id  -- Join with orders
WHERE u.status = 'active'  -- Only active users
GROUP BY u.id, u.username
            "#;

        test_find(
            &format!(r#"sql = """{}""""#, expected_sql),
            vec![("sql", expected_sql)],
            "SQL with comments",
        );
    }

    #[test]
    fn test_stored_procedure_calls() {
        test_find(
            r#"query = "CALL get_user_analytics(?, ?, @result)""#,
            vec![("query", "CALL get_user_analytics(?, ?, @result)")],
            "stored procedure call",
        );
    }

    #[test]
    fn test_ddl_statements() {
        test_find(
            r#"
                query = "CREATE TABLE temp_analytics (id INT PRIMARY KEY, data JSON)"
                sql = "ALTER TABLE users ADD COLUMN last_activity TIMESTAMP"
                query = "DROP TABLE IF EXISTS temp_results"
            "#,
            vec![
                (
                    "query",
                    "CREATE TABLE temp_analytics (id INT PRIMARY KEY, data JSON)",
                ),
                (
                    "sql",
                    "ALTER TABLE users ADD COLUMN last_activity TIMESTAMP",
                ),
                ("query", "DROP TABLE IF EXISTS temp_results"),
            ],
            "DDL statements",
        );
    }

    #[test]
    fn test_empty_sql_string() {
        test_find(r#"sql = """#, vec![("sql", "")], "empty SQL string");
    }

    #[test]
    fn test_annotation_assignment() {
        test_find(
            r#"query: str = "SELECT * FROM users WHERE age > 18""#,
            vec![("query", "SELECT * FROM users WHERE age > 18")],
            "annotated assignment",
        );
    }

    #[test]
    fn test_class_method_assignment() {
        test_find(
            r#"
class UserDAO:
    def __init__(self):
        self.query = "SELECT * FROM users"
        self.sql = "INSERT INTO users (name, email) VALUES (?, ?)"
            "#,
            vec![
                ("query", "SELECT * FROM users"),
                ("sql", "INSERT INTO users (name, email) VALUES (?, ?)"),
            ],
            "class method assignments",
        );
    }

    #[test]
    fn test_function_local_assignment() {
        test_find(
            r#"
def get_users():
    query = "SELECT * FROM users"
    sql = "SELECT * FROM users WHERE active = 1"
    return query
            "#,
            vec![
                ("query", "SELECT * FROM users"),
                ("sql", "SELECT * FROM users WHERE active = 1"),
            ],
            "function local assignments",
        );
    }

    #[test]
    fn test_conditional_assignment() {
        test_find(
            r#"
if condition:
    query = "SELECT * FROM users WHERE role = 'admin'"
else:
    sql = "SELECT * FROM users WHERE role = 'user'"
            "#,
            vec![
                ("query", "SELECT * FROM users WHERE role = 'admin'"),
                ("sql", "SELECT * FROM users WHERE role = 'user'"),
            ],
            "conditional assignments",
        );
    }

    #[test]
    fn test_loop_assignment() {
        test_find(
            r#"
for table in tables:
    # This will be detected:
    sql = "SELECT COUNT(*) FROM table_name"
            "#,
            vec![("sql", "SELECT COUNT(*) FROM table_name")],
            "loop assignments",
        );
    }

    #[test]
    fn test_exception_handling_assignment() {
        test_find(
            r#"
try:
    query = "SELECT * FROM users WHERE complex_condition = true"
except Exception:
    sql = "SELECT * FROM users LIMIT 10"
            "#,
            vec![
                (
                    "query",
                    "SELECT * FROM users WHERE complex_condition = true",
                ),
                ("sql", "SELECT * FROM users LIMIT 10"),
            ],
            "exception handling assignments",
        );
    }

    #[test]
    fn test_global_assignment() {
        test_find(
            r#"
global query
query = "SELECT * FROM global_config"
            "#,
            vec![("query", "SELECT * FROM global_config")],
            "global assignment",
        );
    }

    #[test]
    fn test_mixed_query_and_sql() {
        test_find(
            r#"
query = "SELECT * FROM users"
sql = "INSERT INTO logs (message) VALUES (?)"
query = "UPDATE users SET active = 1"
sql = "DELETE FROM temp_data"
            "#,
            vec![
                ("query", "SELECT * FROM users"),
                ("sql", "INSERT INTO logs (message) VALUES (?)"),
                ("query", "UPDATE users SET active = 1"),
                ("sql", "DELETE FROM temp_data"),
            ],
            "mixed query and sql variables",
        );
    }

    #[test]
    fn test_case_sensitive_patterns() {
        test_find(
            r#"
QUERY = "SELECT * FROM users"
SQL = "INSERT INTO logs VALUES (?)"
Query = "UPDATE users SET status = 'active'"
Sql = "DELETE FROM cache"
            "#,
            vec![
                ("QUERY", "SELECT * FROM users"),
                ("SQL", "INSERT INTO logs VALUES (?)"),
                ("Query", "UPDATE users SET status = 'active'"),
                ("Sql", "DELETE FROM cache"),
            ],
            "case variations of query/sql",
        );
    }

    #[test]
    fn test_complex_nesting_patterns() {
        test_find(
            r#"
((query, sql), (query, sql)) = (("SELECT 1", "SELECT 2"), ("SELECT 3", "SELECT 4"))
            "#,
            vec![
                ("query", "SELECT 1"),
                ("sql", "SELECT 2"),
                ("query", "SELECT 3"),
                ("sql", "SELECT 4"),
            ],
            "complex nested tuple patterns",
        );
    }
}

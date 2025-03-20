/// Test code samples for different languages
pub mod code_samples {
    pub const PYTHON_HELLO: &str = r#"print("Hello from Python!")"#;
    pub const JS_HELLO: &str = r#"console.log('Hello from JavaScript!')"#;
    pub const TS_HELLO: &str = r#"console.log('Hello from TypeScript!')"#;
    pub const GO_HELLO: &str = r#"package main

import "fmt"

func main() {
    fmt.Println("Hello from Go!")
}"#;
    pub const RUST_HELLO: &str = r#"
        fn main() {
            println!("Hello from Rust!");
        }
    "#;
}

/// Test code samples with dependencies
pub mod code_with_deps {
    pub const PYTHON_WITH_DEPS: &str = r#"import numpy as np
arr = np.array([1, 2, 3])
print(f"NumPy sum: {arr.sum()}")"#;

    pub const JS_WITH_DEPS: &str = r#"
        const _ = require('lodash');
        console.log(_.capitalize('hello world'));
    "#;

    pub const GO_WITH_DEPS: &str = r#"package main

import (
    "fmt"
    "github.com/google/uuid"
)

func main() {
    id := uuid.New()
    fmt.Printf("Generated UUID: %s\n", id.String())
}"#;

    pub const RUST_WITH_DEPS: &str = r#"
        use serde_json::json;
        
        fn main() {
            let data = json!({
                "message": "Hello from Rust with serde!"
            });
            println!("{}", data.to_string());
        }
    "#;
}

/// Test code samples for specific scenarios
pub mod test_scenarios {
    pub const PYTHON_MULTILINE: &str = r#"def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)

result = factorial(5)
print(f"Factorial of 5 is {result}")"#;

    pub const PYTHON_WITH_INPUT: &str = r#"name = input()
print(f"Hello, {name}!")"#;

    pub const JS_WITH_TIMEOUT: &str = r#"
        setTimeout(() => {
            console.log('This should not print due to timeout');
        }, 6000);
    "#;

    pub const PYTHON_RESOURCE_HEAVY: &str = r#"# Should be limited by memory constraints
big_list = list(range(10**7))
print(len(big_list))"#;

    pub const GO_WITH_TIMEOUT: &str = r#"package main

import "time"

func main() {
    time.Sleep(10 * time.Second)
}"#;

    pub const RUST_WITH_TIMEOUT: &str = r#"
        use std::thread;
        use std::time::Duration;

        fn main() {
            thread::sleep(Duration::from_secs(10));
        }
    "#;

    pub const GO_RESOURCE_HEAVY: &str = r#"package main

import "fmt"

func main() {
    // Allocate a large slice
    data := make([]int, 10000000)
    for i := range data {
        data[i] = i
    }
    fmt.Println(len(data))
}"#;

    pub const RUST_RESOURCE_HEAVY: &str = r#"
        fn main() {
            // Allocate a large vector
            let data: Vec<i32> = (0..10_000_000).collect();
            println!("Length: {}", data.len());
        }
    "#;
}

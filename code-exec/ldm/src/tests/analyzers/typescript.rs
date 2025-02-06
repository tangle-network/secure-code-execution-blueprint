use crate::{analyze_source_code, Package, PackageSource};

#[tokio::test]
async fn test_typescript_dependency_analysis() {
    let source_code = r#"
        import { useState, useEffect } from 'react';
        import axios from 'axios';
        import type { AxiosResponse } from 'axios';
        import { z } from 'zod';
        import { format } from 'date-fns';

        // Third-party type imports

        import type { NextPage } from 'next';
        import type { QueryClient } from '@tanstack/react-query';

        // Local imports (should be ignored)
        import { MyComponent } from './components';
        import type { MyType } from '../types';

        interface Props {
            data: string[];
        }

        const MyComponent: NextPage<Props> = ({ data }) => {
            const [items, setItems] = useState<string[]>([]);

            useEffect(() => {
                const fetchData = async () => {
                    const response = await axios.get('/api/items');
                    setItems(response.data);
                };
                fetchData();
            }, []);

            return <div>{items.join(', ')}</div>;
        };

        export default MyComponent;
    "#;

    let (lang, deps) = analyze_source_code(source_code).await.unwrap();
    assert_eq!(lang, "typescript");

    // Helper function to find package by name
    fn find_package(name: &str, deps: &[Package]) -> Option<Package> {
        deps.iter().find(|p| p.name == name).cloned()
    }

    // Verify all expected packages are present
    let expected_packages = [
        ("react", Some("^18.0.0")),
        ("axios", Some("^1.0.0")),
        ("zod", Some("3.x")),
        ("date-fns", Some("2.30.0")),
        ("next", Some("13")),
        ("@tanstack/react-query", Some("4")),
    ];

    for (name, version) in expected_packages {
        let pkg = find_package(name, &deps).unwrap_or_else(|| panic!("Package {name} not found"));
        assert_eq!(
            pkg.version.as_deref(),
            version,
            "Version mismatch for {name}"
        );
        assert_eq!(
            pkg.source,
            PackageSource::Custom("npm".to_string()),
            "Source mismatch for {name}"
        );
    }

    // Local imports should not be included
    assert!(find_package("./components", &deps).is_none());
    assert!(find_package("../types", &deps).is_none());
}

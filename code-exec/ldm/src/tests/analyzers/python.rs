use crate::analyze_source_code;

#[tokio::test]
async fn test_python_dependency_analysis() {
    let source_code = r#"
#!/usr/bin/env python3
import json
import boto3
import pandas as pd
from PIL import Image
import numpy as np
from io import BytesIO

# Simple Lambda function that processes an image from S3,
# does some analysis, and stores results in DynamoDB

s3 = boto3.client('s3')
dynamodb = boto3.resource('dynamodb')
table = dynamodb.Table('image-analysis')

def process_image(image_data):
    # Convert image to numpy array for processing
    img = Image.open(BytesIO(image_data))
    img_array = np.array(img)
    
    # Do some basic image analysis
    avg_color = img_array.mean(axis=(0,1))
    brightness = img_array.mean()
    
    # Use pandas for some data manipulation
    results = pd.DataFrame({
        'color_channels': avg_color,
        'brightness': brightness
    })
    
    return results.to_dict()

def lambda_handler(event, context):
    try:
        # Get image from S3
        bucket = event['bucket']
        key = event['key']
        
        response = s3.get_object(Bucket=bucket, Key=key)
        image_data = response['Body'].read()
        
        # Process the image
        results = process_image(image_data)
        
        # Store results in DynamoDB
        table.put_item(Item={
            'image_key': key,
            'analysis': results,
            'timestamp': str(pd.Timestamp.now())
        })
        
        return {
            'statusCode': 200,
            'body': json.dumps(results)
        }
        
    except Exception as e:
        return {
            'statusCode': 500,
            'body': json.dumps({'error': str(e)})
        }
"#;

    let (lang, deps) = analyze_source_code(source_code).await.unwrap();
    assert_eq!(lang, "python");

    let dep_names: Vec<_> = deps.iter().map(|d| d.name.as_str()).collect();
    assert!(dep_names.contains(&"numpy"));
    assert!(dep_names.contains(&"pandas"));
    assert!(dep_names.contains(&"boto3"));
    assert!(dep_names.contains(&"pillow"));

    // Verify version constraints
    let numpy_dep = deps.iter().find(|d| d.name == "numpy").unwrap();
    assert_eq!(numpy_dep.version.as_deref(), Some(">=1.24.0"));
}

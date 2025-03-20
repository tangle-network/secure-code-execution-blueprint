use crate::analyze_source_code;

#[tokio::test]
async fn test_javascript_dependency_analysis() {
    let source_code = r#"
const { DynamoDBClient, QueryCommand } = require('@aws-sdk/client-dynamodb');
const { S3Client, GetObjectCommand } = require('@aws-sdk/client-s3');
const sharp = require('sharp');
const { v4: uuid } = require('uuid');
const axios = require('axios');

// A Lambda function that processes images from S3,
// resizes them, and stores metadata in DynamoDB

const dynamodb = new DynamoDBClient();
const s3 = new S3Client();

// Helper to get weather data for image metadata
async function getWeatherData(lat, lon) {
    const response = await axios.get(
        `https://api.weatherapi.com/v1/current.json?q=${lat},${lon}`
    );
    return response.data;
}

async function processImage(imageBuffer) {
    // Resize image and convert to webp
    const processed = await sharp(imageBuffer)
        .resize(800, 800, { fit: 'inside' })
        .webp({ quality: 80 })
        .toBuffer();

    // Get image metadata
    const metadata = await sharp(imageBuffer).metadata();
    return { processed, metadata };
}

exports.handler = async (event) => {
    try {
        const { bucket, key, latitude, longitude } = JSON.parse(event.body);
        
        // Get image from S3
        const getObjectResponse = await s3.send(
            new GetObjectCommand({
                Bucket: bucket,
                Key: key
            })
        );
        
        // Convert stream to buffer
        const chunks = [];
        for await (const chunk of getObjectResponse.Body) {
            chunks.push(chunk);
        }
        const imageBuffer = Buffer.concat(chunks);
        
        // Process image
        const { processed, metadata } = await processImage(imageBuffer);
        
        // Get weather data
        const weather = await getWeatherData(latitude, longitude);
        
        // Store metadata in DynamoDB
        const imageId = uuid();
        await dynamodb.send(new PutItemCommand({
            TableName: 'processed-images',
            Item: {
                id: { S: imageId },
                originalKey: { S: key },
                width: { N: metadata.width.toString() },
                height: { N: metadata.height.toString() },
                format: { S: metadata.format },
                weather: { M: weather },
                timestamp: { S: new Date().toISOString() }
            }
        }));
        
        return {
            statusCode: 200,
            body: JSON.stringify({
                id: imageId,
                metadata,
                weather
            })
        };
    } catch (error) {
        console.error('Error:', error);
        return {
            statusCode: 500,
            body: JSON.stringify({ error: error.message })
        };
    }
};
"#;

    let (lang, packages) = analyze_source_code(source_code).await.unwrap();
    assert_eq!(lang, "javascript");

    let find_package = |name: &str| packages.iter().find(|p| p.name == name).cloned();

    // Check AWS SDK dependencies
    assert!(find_package("@aws-sdk/client-dynamodb").is_some());
    assert!(find_package("@aws-sdk/client-s3").is_some());

    // Check utility libraries
    assert!(find_package("sharp").is_some());
    assert!(find_package("uuid").is_some());
    assert!(find_package("axios").is_some());

    // Make sure we don't have any unexpected dependencies
    assert_eq!(packages.len(), 5);
}

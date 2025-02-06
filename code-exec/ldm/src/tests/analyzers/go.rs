use crate::analyze_source_code;

#[tokio::test]
async fn test_go_dependency_analysis() {
    let source_code = r#"
package main

import (
    "context"
    "encoding/json"
    "fmt"
    "log"
    "time"

    "github.com/aws/aws-lambda-go/events"
    "github.com/aws/aws-lambda-go/lambda"
    "github.com/aws/aws-sdk-go-v2/config"
    "github.com/aws/aws-sdk-go-v2/service/dynamodb"
    "github.com/aws/aws-sdk-go-v2/service/dynamodb/types"
    "github.com/aws/aws-sdk-go-v2/service/sqs"
    "github.com/google/uuid"
    "go.uber.org/zap"
)

// A Lambda function that processes SQS messages and stores them in DynamoDB
// with additional metadata and logging

type MessageData struct {
    ID        string    `json:"id"`
    UserID    string    `json:"user_id"`
    Content   string    `json:"content"`
    Timestamp time.Time `json:"timestamp"`
}

type ProcessedMessage struct {
    MessageData
    ProcessedAt time.Time `json:"processed_at"`
    MessageID   string    `json:"message_id"`
}

func processMessage(ctx context.Context, msg *sqs.Message) (*ProcessedMessage, error) {
    var data MessageData
    if err := json.Unmarshal([]byte(*msg.Body), &data); err != nil {
        return nil, fmt.Errorf("failed to unmarshal message: %w", err)
    }

    return &ProcessedMessage{
        MessageData:  data,
        ProcessedAt:  time.Now(),
        MessageID:    *msg.MessageId,
    }, nil
}

func handleRequest(ctx context.Context, sqsEvent events.SQSEvent) error {
    // Initialize logger
    logger, _ := zap.NewProduction()
    defer logger.Sync()

    // Load AWS configuration
    cfg, err := config.LoadDefaultConfig(ctx)
    if err != nil {
        return fmt.Errorf("unable to load SDK config: %w", err)
    }

    // Initialize DynamoDB client
    dynamoClient := dynamodb.NewFromConfig(cfg)
    sqsClient := sqs.NewFromConfig(cfg)

    logger.Info("processing messages",
        zap.Int("message_count", len(sqsEvent.Records)))

    for _, record := range sqsEvent.Records {
        msg := &sqs.Message{
            Body:      &record.Body,
            MessageId: &record.MessageId,
        }

        processed, err := processMessage(ctx, msg)
        if err != nil {
            logger.Error("failed to process message",
                zap.String("message_id", record.MessageId),
                zap.Error(err))
            continue
        }

        // Store in DynamoDB
        item, err := json.Marshal(processed)
        if err != nil {
            logger.Error("failed to marshal processed message",
                zap.String("message_id", record.MessageId),
                zap.Error(err))
            continue
        }

        _, err = dynamoClient.PutItem(ctx, &dynamodb.PutItemInput{
            TableName: aws.String("processed-messages"),
            Item: map[string]types.AttributeValue{
                "id": &types.AttributeValueMemberS{Value: uuid.New().String()},
                "message_id": &types.AttributeValueMemberS{Value: record.MessageId},
                "data": &types.AttributeValueMemberS{Value: string(item)},
                "processed_at": &types.AttributeValueMemberS{Value: time.Now().Format(time.RFC3339)},
            },
        })

        if err != nil {
            logger.Error("failed to store message in DynamoDB",
                zap.String("message_id", record.MessageId),
                zap.Error(err))
            continue
        }

        logger.Info("successfully processed message",
            zap.String("message_id", record.MessageId))

        // Delete message from queue
        _, err = sqsClient.DeleteMessage(ctx, &sqs.DeleteMessageInput{
            QueueUrl:      aws.String(record.EventSourceARN),
            ReceiptHandle: aws.String(record.ReceiptHandle),
        })

        if err != nil {
            logger.Error("failed to delete message from SQS",
                zap.String("message_id", record.MessageId),
                zap.Error(err))
        }
    }

    return nil
}

func main() {
    lambda.Start(handleRequest)
}
"#;

    let (lang, packages) = analyze_source_code(source_code).await.unwrap();
    assert_eq!(lang, "go");

    let find_package = |name: &str| packages.iter().find(|p| p.name == name).cloned();

    // Check AWS SDK dependencies
    assert!(find_package("github.com/aws/aws-lambda-go").is_some());
    assert!(find_package("github.com/aws/aws-sdk-go-v2").is_some());

    // Check utility libraries
    assert!(find_package("github.com/google/uuid").is_some());
    assert!(find_package("go.uber.org/zap").is_some());

    // Make sure we don't have any unexpected dependencies
    assert_eq!(packages.len(), 4);

    // Standard library imports should not be included
    let std_packages = ["context", "encoding/json", "fmt", "log", "time"];
    for pkg in std_packages {
        assert!(
            !packages.iter().any(|p| p.name == pkg),
            "Standard library package {} should not be included",
            pkg
        );
    }
}

import * as cdk from "aws-cdk-lib";
import { Construct } from "constructs";
import {
  Function,
  Code,
  Runtime,
  FunctionUrlAuthType,
} from "aws-cdk-lib/aws-lambda";
import * as dynamodb from "aws-cdk-lib/aws-dynamodb";
import * as iam from "aws-cdk-lib/aws-iam";
import path = require("path");

export class DeploymentStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const handler = new Function(this, "RustLambdaFunction", {
      code: Code.fromAsset(
        path.join(
          __dirname,
          "..",
          "..",
          "target/lambda/testing-rust-email-lambda",
        ),
      ),
      runtime: Runtime.PROVIDED_AL2023,
      handler: "whatev",
    });

    const fnUrl = handler.addFunctionUrl({
      authType: FunctionUrlAuthType.NONE,
    });

    const emailTable = new dynamodb.Table(this, "EmailsTable", {
      tableName: "Emails",
      partitionKey: {
        name: "Id",
        type: dynamodb.AttributeType.STRING,
      },
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    handler.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ["dynamodb:Scan", "dynamodb:PutItem"],
        resources: [emailTable.tableArn],
      }),
    );

    handler.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ["ses:SendEmail"],
        resources: ["*"],
      }),
    );

    new cdk.CfnOutput(this, "Lambda URL", {
      value: fnUrl.url,
    });
  }
}

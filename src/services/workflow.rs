// Sapphillon
// Copyright 2025 Yuta Takahashi
//
// This file is part of Sapphillon
//
// Sapphillon is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use fetch::fetch_plugin_package;
use floorp_plugin_browser_info::browser_info_plugin_package;
use sapphillon_core::proto::sapphillon::v1::workflow_service_server::WorkflowService;
use sapphillon_core::proto::sapphillon::v1::{
    FixWorkflowRequest, FixWorkflowResponse, GenerateWorkflowRequest, GenerateWorkflowResponse,
    RunWorkflowRequest, RunWorkflowResponse, Workflow, WorkflowCode,
};
use sapphillon_core::workflow::CoreWorkflowCode;

use crate::workflow::generate_workflow_async;
use std::pin::Pin;
use tokio_stream::Stream;

#[derive(Debug, Default)]
pub struct MyWorkflowService;

#[tonic::async_trait]
impl WorkflowService for MyWorkflowService {
    type FixWorkflowStream = Pin<
        Box<
            dyn Stream<Item = std::result::Result<FixWorkflowResponse, tonic::Status>>
                + Send
                + 'static,
        >,
    >;
    type GenerateWorkflowStream = Pin<
        Box<
            dyn Stream<Item = std::result::Result<GenerateWorkflowResponse, tonic::Status>>
                + Send
                + 'static,
        >,
    >;

    async fn fix_workflow(
        &self,
        request: tonic::Request<FixWorkflowRequest>,
    ) -> std::result::Result<tonic::Response<Self::FixWorkflowStream>, tonic::Status> {
        // 未実装のためエラーを返す
        let _ = request;
        Err(tonic::Status::unimplemented(
            "fix_workflow is not implemented",
        ))
    }

    async fn generate_workflow(
        &self,
        request: tonic::Request<GenerateWorkflowRequest>,
    ) -> std::result::Result<tonic::Response<Self::GenerateWorkflowStream>, tonic::Status> {
        // 未実装のためエラーを返す
        let prompt = request.into_inner().prompt;

        // Generate Workflow Code
        let workflow_code_raw = generate_workflow_async(&prompt)
            .await
            .map_err(|e| tonic::Status::internal(format!("Failed to generate workflow: {e}")))?;
        let workflow_code_raw = workflow_code_raw + "workflow();";

        let workflow_code = WorkflowCode {
            id: uuid::Uuid::new_v4().to_string(),
            code_revision: 1,
            code: workflow_code_raw,
            language: 0,
            created_at: None,
            result: vec![],
            required_permissions: vec![],
            plugin_packages: vec![],
            plugin_function_ids: vec![],
        };

        let workflow = Workflow {
            id: uuid::Uuid::new_v4().to_string(),
            display_name: "Generated Workflow".to_string(),
            description: "This is a generated workflow".to_string(),
            workflow_language: 0,
            workflow_code: vec![workflow_code],
            created_at: None,
            updated_at: None,
            workflow_results: vec![],
        };

        let response = GenerateWorkflowResponse {
            workflow_definition: Some(workflow),
            status: Some(sapphillon_core::proto::google::rpc::Status {
                code: 0,
                message: "Workflow generated successfully".to_string(),
                details: vec![],
            }),
        };

        // return the response
        // stream the single response back to the client
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        // move the response into a background task so we can return a stream immediately
        tokio::spawn(async move {
            // send the response; ignore error if receiver was dropped
            let _ = tx.send(Ok(response)).await;
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let boxed_stream: Self::GenerateWorkflowStream =
            Box::pin(stream) as Self::GenerateWorkflowStream;

        Ok(tonic::Response::new(boxed_stream))
    }

    async fn run_workflow(
        &self,
        request: tonic::Request<RunWorkflowRequest>,
    ) -> std::result::Result<tonic::Response<RunWorkflowResponse>, tonic::Status> {
        let mut workflow = match request.into_inner().workflow_definition.clone() {
            Some(workflow) => workflow.clone(),
            None => {
                return Err(tonic::Status::invalid_argument(
                    "Workflow definition is required",
                ));
            }
        };
        let latest_workflow_code_revision = workflow
            .workflow_code
            .iter()
            .map(|code| code.code_revision)
            .max()
            .unwrap_or(0);

        let workflow_code = workflow
            .workflow_code
            .iter_mut()
            .find(|code| code.code_revision == latest_workflow_code_revision)
            .ok_or_else(|| tonic::Status::not_found("Latest workflow code not found"))?;
        workflow_code.code = unescaper::unescape(&workflow_code.code).unwrap();

        log::debug!("Parsed workflow code: {}", workflow_code.code);

        // Offload Deno execution to a blocking thread and rebuild CoreWorkflowCode in that thread
        // to avoid moving non-Send types across await points.
        let code_id = workflow_code.id.clone();
        let code_body = workflow_code.code.clone();
        let code_rev = workflow_code.code_revision;

        let latest_result = tokio::task::spawn_blocking(move || {
            let mut workflow_core = CoreWorkflowCode::new(
                code_id,
                code_body,
                vec![
                    fetch_plugin_package(),
                    browser_info_plugin_package(),
                ],
                code_rev,
            );
            workflow_core.run();

            let latest_result_revision = workflow_core
                .result
                .iter()
                .map(|r| r.workflow_result_revision)
                .max()
                .unwrap_or(0);

            workflow_core
                .result
                .into_iter()
                .find(|r| r.workflow_result_revision == latest_result_revision)
                .unwrap()
        })
        .await
        .map_err(|e| tonic::Status::internal(format!("Join error: {e}")))?;

        let res = RunWorkflowResponse {
            workflow_result: Some(latest_result),
            status: Some(sapphillon_core::proto::google::rpc::Status {
                code: 0,
                message: "Workflow executed successfully".to_string(),
                details: vec![],
            }),
        };

        // Return the response
        Ok(tonic::Response::new(res))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sapphillon_core::proto::sapphillon::v1::{WorkflowCode, Workflow};

    #[tokio::test]
    async fn test_run_workflow_simple_console() {
        let svc = MyWorkflowService::default();

        // Minimal workflow that prints a line
        let code = r#"function workflow(){ console.log('OK'); return 'OK'; }workflow();"#;
        let wf_code = WorkflowCode {
            id: "wid".to_string(),
            code_revision: 1,
            code: code.to_string(),
            ..Default::default()
        };
        let wf = Workflow {
            id: "w".to_string(),
            display_name: "t".to_string(),
            description: "d".to_string(),
            workflow_language: 0,
            workflow_code: vec![wf_code],
            ..Default::default()
        };
        let req = RunWorkflowRequest { workflow_definition: Some(wf) };
        let resp = svc.run_workflow(tonic::Request::new(req)).await.unwrap().into_inner();
        assert_eq!(resp.status.unwrap().code, 0);
        let out = resp.workflow_result.unwrap().result;
        assert!(out.contains("OK"));
    }
}

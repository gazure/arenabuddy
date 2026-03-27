use arenabuddy_core::services::debug_service::{
    ReportParseErrorsRequest, ReportParseErrorsResponse, debug_service_server::DebugService,
};
use arenabuddy_data::{DebugRepository, MatchDB};
use tonic::{Request, Response, Status};
use tracing::{error, info, instrument};

use crate::auth::UserId;

pub(crate) struct DebugServiceImpl {
    pub(crate) db: MatchDB,
}

#[tonic::async_trait]
impl DebugService for DebugServiceImpl {
    #[instrument(skip(self, request))]
    async fn report_parse_errors(
        &self,
        request: Request<ReportParseErrorsRequest>,
    ) -> Result<Response<ReportParseErrorsResponse>, Status> {
        let user_id = request.extensions().get::<UserId>().map(|u| u.0);
        let errors = request.into_inner().errors;

        let mut accepted = 0i32;
        for err in &errors {
            let reported_at = chrono::DateTime::from_timestamp(err.timestamp, 0).unwrap_or_else(chrono::Utc::now);

            self.db
                .insert_parse_error(user_id, &err.raw_json, reported_at)
                .await
                .map_err(|e| {
                    error!("Failed to insert parse error: {e}");
                    Status::internal("failed to store parse error")
                })?;
            accepted += 1;
        }

        info!("Stored {accepted} parse error(s) from user {user_id:?}");
        Ok(Response::new(ReportParseErrorsResponse { accepted }))
    }
}

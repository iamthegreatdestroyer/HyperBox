//! gRPC API server.

use crate::state::DaemonState;
use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::info;

/// Serve the gRPC API.
pub async fn serve(state: DaemonState, addr: SocketAddr) -> anyhow::Result<()> {
    info!("gRPC API listening on {}", addr);

    // For now, a placeholder. Full implementation would include:
    // - Container service
    // - Image service
    // - Project service
    // - System service
    // Generated from proto files with tonic-build

    // Placeholder server
    tokio::time::sleep(std::time::Duration::from_secs(u64::MAX)).await;

    Ok(())
}

// Proto definitions would go in proto/hyperbox.proto:
//
// syntax = "proto3";
// package hyperbox.v1;
//
// service ContainerService {
//   rpc List(ListContainersRequest) returns (ListContainersResponse);
//   rpc Create(CreateContainerRequest) returns (Container);
//   rpc Start(StartContainerRequest) returns (Empty);
//   rpc Stop(StopContainerRequest) returns (Empty);
//   rpc Remove(RemoveContainerRequest) returns (Empty);
//   rpc Checkpoint(CheckpointRequest) returns (Checkpoint);
//   rpc Restore(RestoreRequest) returns (Container);
// }
//
// service ImageService {
//   rpc List(ListImagesRequest) returns (ListImagesResponse);
//   rpc Pull(PullImageRequest) returns (stream PullProgress);
//   rpc Remove(RemoveImageRequest) returns (Empty);
// }
//
// service ProjectService {
//   rpc List(ListProjectsRequest) returns (ListProjectsResponse);
//   rpc Open(OpenProjectRequest) returns (Project);
//   rpc Start(StartProjectRequest) returns (Empty);
//   rpc Stop(StopProjectRequest) returns (Empty);
//   rpc Close(CloseProjectRequest) returns (Empty);
// }
//
// service SystemService {
//   rpc Info(Empty) returns (SystemInfo);
//   rpc Version(Empty) returns (VersionInfo);
//   rpc Events(EventsRequest) returns (stream Event);
//   rpc Metrics(Empty) returns (Metrics);
// }

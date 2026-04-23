use crate::models::Deployment;

pub trait Builder: Send + Sync {
    fn build(
        &self,
        deployment: &Deployment,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;
}

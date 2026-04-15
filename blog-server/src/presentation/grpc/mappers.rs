use crate::presentation::grpc::proto::blog;

impl From<crate::domain::user::User> for blog::User {
    fn from(u: crate::domain::user::User) -> Self {
        blog::User {
            id: u.id,
            username: u.username,
            email: u.email,
            created_at: Some(prost_types::Timestamp {
                seconds: u.created_at.timestamp(),
                nanos: u.created_at.timestamp_subsec_nanos() as i32,
            }),
        }
    }
}
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

impl From<crate::domain::post::Post> for blog::Post {
    fn from(p: crate::domain::post::Post) -> Self {
        blog::Post {
            id: p.id,
            author_id: p.author_id,
            title: p.title,
            content: p.content,
            created_at: Some(prost_types::Timestamp {
                seconds: p.created_at.timestamp(),
                nanos: p.created_at.timestamp_subsec_nanos() as i32,
            }),
            updated_at: Some(prost_types::Timestamp {
                seconds: p.updated_at.timestamp(),
                nanos: p.updated_at.timestamp_subsec_nanos() as i32,
            }),
        }
    }
}

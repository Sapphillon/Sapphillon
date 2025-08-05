// Re-export the generated protobuf code
pub mod sapphillon {
    #![allow(clippy::all)]
    pub mod v1 {
        include!("proto_generated/sapphillon.v1.rs");
    }
}

pub mod google {
    #![allow(clippy::all)]
    pub mod api {
        include!("proto_generated/google.api.rs");
        pub mod expr {
            pub mod v1alpha1 {
                include!("proto_generated/google.api.expr.v1alpha1.rs");
            }
            pub mod v1beta1 {
                include!("proto_generated/google.api.expr.v1beta1.rs");
            }
        }
    }
    pub mod bytestream {
        include!("proto_generated/google.bytestream.rs");
    }
    pub mod longrunning {
        include!("proto_generated/google.longrunning.rs");
    }
    pub mod geo {
        pub mod r#type {
            include!("proto_generated/google.geo.type.rs");
        }
    }
    pub mod rpc {
        include!("proto_generated/google.rpc.rs");
        pub mod context {
            include!("proto_generated/google.rpc.context.rs");
        }
    }
    pub mod r#type {
        include!("proto_generated/google.type.rs");
    }
}

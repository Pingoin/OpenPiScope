use open_pi_scope::{alignment::Orientation, gnss, magnetic::MagneticData};
use utoipa_axum::{routes,  router::OpenApiRouter};
use axum::{response::{IntoResponse, Response}, Json};
use tokio::net::TcpListener;
use utoipa_swagger_ui::SwaggerUi;

use crate::storage::storage;


pub(crate) async fn handle_web() -> anyhow::Result<()> {



let (router, api) = OpenApiRouter::new()
    .routes(routes!(get_gnss_data))
    .routes(routes!(magnetic_data))
    .routes(routes!(alignment_data))
    .split_for_parts();

    let router = router.merge(SwaggerUi::new("/swagger-ui").url("/apidoc/openapi.json", api));

    let listener = TcpListener::bind(("0.0.0.0", 8080)).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/gnss-data",
    responses(
        (status = 200, description = "GNSS data retrieved successfully", body = gnss::GnssData),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_gnss_data()->Response{
    let storage = storage();
    let gnss_data=storage.get_gnss_data().await;
   Json(&gnss_data).into_response()
}

#[utoipa::path(
    get,
    path = "/api/magnetic-data",
    responses(
        (status = 200, description = "GNSS data retrieved successfully", body = MagneticData),
        (status = 500, description = "Internal server error")
    )
)]
async fn magnetic_data()->Response{
    let storage = storage();
    let magnetic_data=storage.get_magnetic_data().await;
   Json(&magnetic_data).into_response()
}


#[utoipa::path(
    get,
    path = "/api/alignment",
    responses(
        (status = 200, description = "Orientation data retrieved successfully", body = Orientation),
        (status = 500, description = "Internal server error")
    )
)]
async fn alignment_data()->Response{
    let storage = storage();
    let orientation_data=storage.get_orientation().await;
   Json(&orientation_data).into_response()
}   
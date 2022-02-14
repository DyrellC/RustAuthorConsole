use hyper::{Request, Body, Response, StatusCode, header};
use crate::streams::ChannelAuthor;
use crate::models::{SubscriptionRequest, ReadingWrapper, AnnotationWrapper};
use std::sync::{Arc};
use parking_lot::Mutex;
use iota_streams::app::futures::executor::block_on;
use mysql::params;
use mysql::prelude::Queryable;
use crate::store::{ReadingStoreFilterId, AnnotationStoreFilterId};

type GenericError = Box<dyn std::error::Error + Send + Sync>;

pub async fn preflight_response(
) -> Result<Response<Body>, GenericError> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS, PUT, PATCH, DELETE")
        .header("Access-Control-Allow-Headers", "Content-Type")
        .body(Body::from("OK"))
        .unwrap())
}

pub async fn subscribe_response(
    req: Request<Body>,
    author: Arc<Mutex<ChannelAuthor>>,
) -> Result<Response<Body>, GenericError> {
    let data = hyper::body::to_bytes(req.into_body()).await?;

    let response;
    let json_data: serde_json::Result<SubscriptionRequest> = serde_json::from_slice(&data);

    if let Ok(sub_req) = json_data {
        let mut author = author.lock();
        let pk = hex::decode(sub_req.pk)?;
        let msgid = sub_req.msgid.clone();
        let keyload = tokio::task::block_in_place(|| {block_on(author.subscribe(&msgid, &pk))});
        match keyload {
            Ok(keyload_link) => {
                println!("Processed subscription, returning keyload link...");
                response = Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Body::from("Subscription processed, keyload link: ".to_owned() + &keyload_link.to_string()))?;
            },
            Err(_) => {
                response = Response::builder()
                    .status(500)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Body::from("Error while subscribing"))?;
            }
        }
    } else {
        dbg!("Error in formatting");
        response = Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(header::CONTENT_TYPE, "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .body(Body::from("Malformed json request"))?;
    }

    Ok(response)
}

pub async fn channel_address_response(
    author: Arc<Mutex<ChannelAuthor>>,
) -> Result<Response<Body>, GenericError> {
    let response;

    let author = author.lock();
    match author.get_channel_address() {
        Ok(channel_address) => {
            response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from("{ \"channel_address\": \"".to_owned() + &channel_address + "\" }"))?;
        },
        Err(_e) => {
            response = Response::builder()
                .status(500)
                .header(header::CONTENT_TYPE, "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from("Error while getting channel address"))?;
        }
    }

    Ok(response)
}

pub async fn announcement_id_response(
    author: Arc<Mutex<ChannelAuthor>>,
) -> Result<Response<Body>, GenericError> {
    let response;

    let author = author.lock();
    match author.get_announcement_id() {
        Ok(announcement_id) => {
            response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from("{ \"announcement_id\": \"".to_owned() + &announcement_id.0 +
                    ":" + &announcement_id.1.to_owned() + "\" }"))?;
        },
        Err(_e) => {
            response = Response::builder()
                .status(500)
                .header(header::CONTENT_TYPE, "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from("Error while getting announcement id"))?;
        }
    }

    Ok(response)
}

pub async fn readings_response(
    req: Request<Body>,
    reading_store: mysql::Pool
) -> Result<Response<Body>, GenericError> {
    let data = hyper::body::to_bytes(req.into_body()).await?;

    let mut response= Response::builder()
        .status(500)
        .header(header::CONTENT_TYPE, "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from("Error while reading sensor_id field"))?;

    let sensor_id: serde_json::Result<ReadingStoreFilterId> = serde_json::from_slice(&data);
    if let Ok(sensor_id) = sensor_id {
        match reading_store.get_conn() {
            Ok(mut conn) => {
                response = Response::builder()
                    .status(500)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Body::from("Error retrieving from sql db"))?;

                let mut readings = Vec::new();
                if let Ok(_) = conn.exec_map("SELECT * FROM molina.readings WHERE sensor_id = :sensor_id",
                                             params! {"sensor_id" => sensor_id.get_sensor_id()},
                                             |(sensor_id, reading_id, reading)| {
                                                 readings.push(ReadingWrapper { sensor_id, reading_id, reading })
                                             }
                ) {
                    if let Ok(_) = conn.exec_map("SELECT * FROM molina.sheet_readings WHERE sheet_id = :sheet_id",
                                                 params! {"sheet_id" => sensor_id.get_sensor_id()},
                                                 |(sensor_id, reading_id, reading)| {
                                                     readings.push(ReadingWrapper { sensor_id, reading_id, reading })
                                                 }) {
                        response = Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .header("Access-Control-Allow-Origin", "*")
                            .body(Body::from(serde_json::to_string(&readings)?))?;
                    }
                }
            }
            Err(_) => {
                response = Response::builder()
                    .status(500)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Body::from("Error getting sql connection"))?;
            }
        }
    }
    Ok(response)
}

pub async fn annotations_response(
    req: Request<Body>,
    sql: mysql::Pool,
) -> Result<Response<Body>, GenericError> {
    let data = hyper::body::to_bytes(req.into_body()).await?;

    let mut response= Response::builder()
        .status(500)
        .header(header::CONTENT_TYPE, "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from("Error while reading reading_id field"))?;

    let reading_id: serde_json::Result<AnnotationStoreFilterId> = serde_json::from_slice(&data);
    if let Ok(reading_id) = reading_id {
        match sql.get_conn() {
            Ok(mut conn) => {
                response = Response::builder()
                    .status(500)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Body::from("Error retrieving annotations from sql db"))?;

                let mut annotations = Vec::new();
                if let Ok(_) = conn.exec_map("SELECT * FROM molina.annotations WHERE reading_id = :reading_id",
                                             params! {"reading_id" => reading_id.get_reading_id()},
                                             |(reading_id, annotation)| {
                                                 annotations.push(AnnotationWrapper { reading_id, annotation })
                                             }
                ) {       response = Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .header("Access-Control-Allow-Origin", "*")
                            .body(Body::from(serde_json::to_string(&annotations)?))?;
                    }
            }
            Err(_) => {
                response = Response::builder()
                    .status(500)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Body::from("Error getting sql connection"))?;
            }
        }
    }
    Ok(response)
}

/*pub async fn filter_annotations_response(
    req: Request<Body>,
    annotation_store: Arc<Mutex<AnnotationStore>>
) -> Result<Response<Body>, GenericError> {
    let data = hyper::body::to_bytes(req.into_body()).await?;

    let response;
    let filters: serde_json::Result<AnnotationStoreFilter> = serde_json::from_slice(&data);

    match filters {
        Ok(filters) => {
            //let mut annotation_store = annotation_store.lock();
            //match annotation_store.lock() {
            //    Ok(mut annotation_store) => {
                    //let mut annotations = Vec::new();

/*                    let mapped = annotation_store.iter().map(|ann_iter| {
                        ann_iter.for_each(|(_id, ann_vec)| {
                            annotations.extend(ann_vec.into_iter()
                                .filter(|ann| filters.iss.is_some() &&
                                    &ann.annotation.payload.iss == filters.iss.as_ref().unwrap()));
                            annotations.extend(ann_vec.into_iter()
                                .filter(|ann| filters.ann.is_some() &&
                                    &ann.annotation.payload.ann == filters.ann.as_ref().unwrap()));
                            annotations.extend(ann_vec.into_iter()
                                .filter(|ann| filters.iat.is_some() &&
                                    &ann.annotation.payload.iat == filters.iat.as_ref().unwrap()));
                            annotations.extend(ann_vec.into_iter()
                                .filter(|ann| filters.jti.is_some() &&
                                    &ann.annotation.payload.jti == filters.jti.as_ref().unwrap()));
                            annotations.extend(ann_vec.into_iter()
                                .filter(|ann| filters.sub.is_some() &&
                                    &ann.annotation.payload.sub == filters.sub.as_ref().unwrap()));
                        })
                    });

                    if mapped.is_ok() {
                        response = Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .header("Access-Control-Allow-Origin", "*")
                            .body(Body::from(serde_json::to_vec(&annotations)?))?;
                    } else {*/
                        response = Response::builder()
                            .status(500)
                            .header(header::CONTENT_TYPE, "application/json")
                            .header("Access-Control-Allow-Origin", "*")
                            .body(Body::from("Error while mapping annotations"))?;
                    //}
                /*},
                _ => {
                    response = busy()
                }
            }*/
        },

        Err(_) => {
            response = Response::builder()
                .status(500)
                .header(header::CONTENT_TYPE, "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Body::from("Error while reading filters fields"))?;
        }
    }

    Ok(response)
}*/

/*fn busy() -> Response<Body>{
    Response::builder()
        .status(500)
        .header(header::CONTENT_TYPE, "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from("Service is busy"))
        .unwrap()
}*/

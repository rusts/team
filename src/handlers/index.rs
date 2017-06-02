use persistent;
use iron::prelude::*;
use iron::status;
use iron::modifiers::{Redirect};
use hbs::{Template};
use hbs::handlebars::to_json;

use db;
use models;
use handlers;

const PAGINATES_PER: i32 = 2;

pub fn index_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let conn_l = get_pg_connection!(req);
    let conn_c = get_pg_connection!(req);

    let page_param: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();
        match map.get("page") {
            Some(&Value::String(ref name)) => {
                page_param = name.to_string();
            },
            _ => page_param = "1".to_string(),
        }
    }

    let mut resp = Response::new();

    #[derive(Serialize, Debug)]
    struct Data {
        logged_in: bool,
        posts: Vec<models::post::Post>,
        total_count: i32,
        next_page: i32,
        prev_page: i32,
    }

    let mut page = page_param.parse::<i32>().unwrap();
    let offset = ( page - 1 ) * PAGINATES_PER;
    let limit = PAGINATES_PER;

    let posts: Vec<models::post::Post>;
    let count: i32;

    match models::post::list_all(conn_l, offset, limit) {
        Ok(posts_db) => {
            posts = posts_db;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::count_all(conn_c) {
        Ok(count_db) => {
            count = count_db;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    if page == 0 {
        page = 1;
    }
    let data = Data {
        logged_in: login_id != 0,
        posts: posts,
        total_count: count,
        next_page: page + 1,
        prev_page: page - 1,
    };

    resp.set_mut(Template::new("index", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);

    // let mut resp = Response::new();
    // let mut data = HashMap::new();
    // data.insert(String::from("title"), "Team".to_string());
    // resp.set_mut(Template::new("index", data)).set_mut(status::Ok);
    // return Ok(resp);
}
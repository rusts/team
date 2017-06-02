use persistent;
use iron::prelude::*;
use iron::status;
use router::{Router};
use hbs::{Template};
use iron::modifiers::{Redirect};
use hbs::handlebars::to_json;
use iron::{Url};
use iron::prelude::{IronResult};

use db;
use helper;
use models;
use handlers;

const PAGINATES_PER: i32 = 10;
const POST_KIND: i32 = 2;

pub fn new_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let mut resp = Response::new();

    #[derive(Serialize)]
    struct Data {
        logged_in: bool,
    }
    let data = Data {
        logged_in: login_id != 0,
    };
    resp.set_mut(helper::template("nippo/form", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}

pub fn create_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let conn = get_pg_connection!(req);

    let title: String;
    let body: String;

    {
        use params::{Params, Value};
        let map = req.get_ref::<Params>().unwrap();
        match map.get("title") {
            Some(&Value::String(ref name)) => {
                title = name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
        match map.get("body") {
            Some(&Value::String(ref name)) => {
                body = name.to_string();
            },
            _ => return Ok(Response::with((status::BadRequest))),
        }
    }

    match models::post::create(conn, POST_KIND, login_id, title, body) {
        Ok(id) => {
            let url = Url::parse(&format!("{}/{}/{}", helper::get_domain(), "nippo/show", id).to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    }
}

pub fn list_handler(req: &mut Request) -> IronResult<Response> {
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

    match models::post::list(conn_l, 2, offset, limit) {
        Ok(posts_db) => {
            posts = posts_db;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::count(conn_c, POST_KIND) {
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

    resp.set_mut(Template::new("nippo/list", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}

pub fn show_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn_s = get_pg_connection!(req);
    let conn_c = get_pg_connection!(req);
    let mut resp = Response::new();

    let ref id_str = req.extensions.get::<Router>().unwrap().find("id").unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    #[derive(Serialize, Default)]
    struct Data {
        logged_in: bool,
        post: models::post::Post,
        editable: bool,
        comments: Vec<models::post::Comment>,
    }

    let post: models::post::Post;
    let comments: Vec<models::post::Comment>;

    match models::post::get_marked_by_id(conn_s, id) {
        Ok(post_db) => {
            post = post_db;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    }

    match models::post::get_comments_by_post_id(conn_c, id) {
        Ok(comments_db) => {
            comments = comments_db;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)))
        }
    }

    let owner_id = post.user_id;
    let data = Data {
        logged_in: login_id != 0,
        post: post,
        editable: owner_id == login_id,
        comments: comments,
    };

    resp.set_mut(Template::new("nippo/show", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}

pub fn delete_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn_s = get_pg_connection!(req);
    let conn_d = get_pg_connection!(req);

    let ref id_str = req.extensions.get::<Router>().unwrap().find("id").unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    match models::post::get_by_id(conn_s, id) {
        Ok(nippo) => {
            if nippo.user_id != login_id {
                return Ok(Response::with((status::Forbidden)));
            }
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::delete_by_id(conn_d, id) {
        Ok(_) => {
            return Ok(Response::with((status::Found, Redirect(url_for!(req, "nippo/list")))));
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            Ok(Response::with((status::InternalServerError)))
        }
    }
}

pub fn edit_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn = get_pg_connection!(req);
    let mut resp = Response::new();
    #[derive(Serialize, Default)]
    struct Data {
        logged_in: bool,
        post: models::post::Post,
    }

    let post: models::post::Post;

    let ref id_str = req.extensions.get::<Router>().unwrap().find("id").unwrap_or("/");
    let id = id_str.parse::<i32>().unwrap();

    match models::post::get_by_id(conn, id) {
        Ok(post_db) => {
            if post_db.user_id != login_id {
                return Ok(Response::with((status::Forbidden)));
            }
            post = post_db;
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
    let data = Data {
        logged_in: login_id != 0,
        post: post,
    };
    resp.set_mut(Template::new("nippo/edit", to_json(&data))).set_mut(status::Ok);
    return Ok(resp);
}

pub fn update_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }

    let conn_s = get_pg_connection!(req);
    let conn_u = get_pg_connection!(req);

    use params::{Params, Value};
    let map = req.get_ref::<Params>().unwrap();

    let id: i32;
    let title: String;
    let body: String;

    match map.find(&["id"]) {
        Some(&Value::String(ref name)) => {
            id = name.to_string().parse::<i32>().unwrap();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["title"]) {
        Some(&Value::String(ref name)) => {
            title = name.to_string();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["body"]) {
        Some(&Value::String(ref name)) => {
            body = name.to_string();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match models::post::get_by_id(conn_s, id) {
        Ok(post_db) => {
            if post_db.user_id != login_id {
                return Ok(Response::with((status::Forbidden)));
            }
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }

    match models::post::update(conn_u, id, title, body) {
        Ok(_) => {
            let url = Url::parse(&format!("{}/{}/{}", helper::get_domain(), "nippo/show", id).to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
}

pub fn comment_handler(req: &mut Request) -> IronResult<Response> {
    let login_id = handlers::account::get_login_id(req);
    if login_id == 0 {
        return Ok(Response::with((status::Found, Redirect(url_for!(req, "account/get_signin")))));
    }
    let conn = get_pg_connection!(req);

    use params::{Params, Value};
    let map = req.get_ref::<Params>().unwrap();

    let id: i32;
    let body: String;

    match map.find(&["id"]) {
        Some(&Value::String(ref name)) => {
            id = name.parse::<i32>().unwrap();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match map.find(&["body"]) {
        Some(&Value::String(ref name)) => {
            body = name.to_string();
        },
        _ => return Ok(Response::with((status::BadRequest))),
    }

    match models::post::add_comment(conn, login_id, id, body) {
        Ok(_) => {
            let url = Url::parse(&format!("{}/{}/{}", helper::get_domain(), "nippo/show", id).to_string()).unwrap();
            return Ok(Response::with((status::Found, Redirect(url))));
        },
        Err(e) => {
            println!("Errored: {:?}", e);
            return Ok(Response::with((status::InternalServerError)));
        }
    }
}
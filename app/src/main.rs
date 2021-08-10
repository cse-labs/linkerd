#[macro_use] extern crate rocket;

#[get("/")]
fn index() -> &'static str {
    "root"
}

#[get("/v1.0")]
fn v1_0() -> &'static str {
    "v1.0"
}

#[get("/v1.0/me")]
fn me() -> &'static str {
    "myself"
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, v1_0, me])
}

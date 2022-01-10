use serde::{Serialize, Deserialize};
use worker::*;
use urlencoding;
use std::fs;

mod utils;

#[derive(Serialize, Deserialize)]
struct Post {
    title: String,
    user: String,
    content: String,
    likes: u32,
    image: String
}


#[event(fetch)]
pub async fn main(req: Request, env: Env) -> Result<Response> {
    // log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get_async("/posts", |_req, ctx| async move {
            let post_kv = ctx.kv("posts")?;
            let mut posts: Vec<Post> = vec![];

            // Getting the number of posts
            let num_posts: u32 = if let Some(post_num) = post_kv.get("posts").await? {
                post_num.as_string().parse().unwrap()
            } else { 0 };
            // Parsing previously stored posts
            for n in 0..num_posts {
                posts.push(serde_json::from_str(&post_kv.get(&n.to_string()).await?.unwrap().as_string()).unwrap());
            }
            return Response::from_json(&serde_json::to_string(&posts).unwrap());
        })
        // .get_async("/post/:id", |_req, ctx| async move {
        //     This would work better imo, but it's not what the instructions ask for
        // })
        .post_async("/posts", |mut req, ctx| async move {
            let post_kv = ctx.kv("posts")?;
            // Getting the number of posts
            let num_posts: u32 = if let Some(post_num) = post_kv.get("posts").await? {
                post_num.as_string().parse().unwrap()
            } else { 0 };
            let form = req.form_data().await?;
            let mut post = Post {
                title: "".into(),
                user: "".into(),
                content: "".into(),
                likes: 0u32,
                image: "".into()
            };
            match form.get("title") {
                Some(FormEntry::Field(value)) => {
                    post.title = value;
                }
                _ => return Response::error("Bad Request", 400),
            }
            match form.get("user") {
                Some(FormEntry::Field(value)) => {
                    post.user = value;
                }
                _ => return Response::error("Bad Request", 400),
            }
            match form.get("content") {
                Some(FormEntry::Field(value)) => {
                    post.content = value;
                }
                _ => return Response::error("Bad Request", 400),
            }
            match form.get("image-url") {
                Some(FormEntry::Field(value)) => {
                    post.image = urlencoding::encode(&value);
                }
                _ => {
                    post.image = "".into();
                }
            }
            post_kv.put(&num_posts.to_string(), &serde_json::to_string(&post).unwrap())?.execute().await?;
            let new_num_posts = num_posts + 1;
            post_kv.put("posts", &new_num_posts.to_string())?.execute().await?;
            Response::ok("success")
        })
        .post_async("/like", |mut req, ctx| async move {
            // if this was used in production it would get spammed because there is no check
            //      that the request comes from a legitimate user
            let post_kv = ctx.kv("posts")?;
            let post_id = req.text().await?;
            let mut post: Post = serde_json::from_str(&post_kv.get(&post_id).await?.unwrap().as_string()).unwrap();
            post.likes += 1;
            post_kv.put(&post_id, &serde_json::to_string(&post).unwrap())?.execute().await?;
            Response::ok(&post.likes.to_string())
        })
        .run(req, env)
        .await
}

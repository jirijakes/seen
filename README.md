# Seen

## About

If you are trying to recall a webpage you have seen before and you only remember a few keywords
and perhaps also predominant color, *`seen`* will help you find it.

## Features

Only basic form of the features is available now.

 - [X] Download and index web page
 - [X] Specify tags
 - [X] Search by content, tags and domain
 - [X] Display content of indexed web pages
 - [ ] Export indexed pages to static website
 - [ ] Store indexed web pages as PDF and image
 - [ ] Index speech in videos
 - [ ] Index text in images
 - [ ] Fire-and-forget indexing
 - [ ] Expose interface for web browser extensions

## Try now

```
fossil clone https://jirijakes.com/code/seen
make clean
cargo run -- add -t personality https://www.maxcountryman.com/articles/grow-in-public
cargo run -- search team
cargo run -- search "tag:personality"
```

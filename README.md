# Seen

## About

Have you ever tried to recall an interesting article you had read but you could only remember a few keywords
and background color of the web page? Or that it was very long? Or that you read it one winter evening?
*Seen* can help you find it again!

## Warning

At the moment, nothing really works properly. Better do not use it yet.

## Features

Only basic form of the features is available now.

 - [X] Download and index web page
 - [X] Specify tags
 - [X] Search by content, tags, time and domain
 - [X] Display content of indexed web pages
 - [ ] Export indexed pages to static website
 - [ ] Store indexed web pages as PDF and image
 - [ ] Index speech in videos
 - [ ] Index text in images
 - [ ] Fire-and-forget indexing
 - [ ] Expose interface for web browser extensions
 - [ ] Search by other attributes (colors, language, length, …)
 - …

## Try now

```
fossil clone https://jirijakes.com/code/seen
cd seen
make clean
cargo install --path .
```

```
seen add -t personality https://www.maxcountryman.com/articles/grow-in-public
seen search team
seen search "tag:personality"
seen list
seen get <UUID>
```

Fox Friday Bot
==============

A bot that posts a video of foxes. 

Primarily uses **reqwest** for its HTTP calls. The oAuth authentication header
generation was written by me, as well as the API wrapper around the Twitter
(v1.1) API. 

The proper way of deploying it is impossible, except for me (the bot is
hard-coded to upload the fox friday video from a S3 bucket) but you can 
change a couple of things in `main.rs` in order to modify it to do what 
you want to do.

The primary flow is this:
1. Bot fetches `fox_friday.mp4` from a S3 bucket and stores it into memory
2. Bot feeds `fox_friday.mp4` into a function that chunks the video, and sends
   it to Twitter after calling the proper APIs
3. Bot sends a tweet with the video within it in order to post it onto its
   Twitter account.

This is done via a Lambda function, which is triggered by a cron expression that
runs it every friday at 4 AM UTC (9 AM PST).

This was mostly for me to learn how to effectively generate an oAuth
authorization header (and it still has hiccups on my local machine when I was
testing), as well as practice some more Rust in a practical sense.

License
=======

Dual licensed under the MIT and Apache 2.0 Licenses. Copyright 2021 Flipp Syder.
See LICENSE-MIT and LICENSE-APACHE for details.

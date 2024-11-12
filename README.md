![GitHub Release](https://img.shields.io/github/v/release/Diegovsky/htmeta)
![GitHub Repo stars](https://img.shields.io/github/stars/Diegovsky/htmeta)
![GitHub Forks](https://img.shields.io/github/forks/Diegovsky/htmeta)
![GitHub Contributors](https://img.shields.io/github/contributors/Diegovsky/htmeta)


This crates implements a (simple) flavour of [`KDL`] called `htmeta`. This
allows you to write `HTML` in a better, more comfortable and simpler format than
raw `HTML`!

Mainly, this gets rid of the tag pairs `<>` and `</>`, in favour of good ol' curly brackets.
Here's the same page written in both `HTML` and `htmeta`: 

```html
<!DOCTYPE html>
<html>
    <body>
        <h1>Welcome!</h1>
        <p>
            This is an example page. Very cool!
        </p>
    </body>
</html>
```

```kdl
!DOCTYPE html
html {
    body {
        h1 text="Welcome!"
        p {
            text "This is an example page. Very cool!"
        }
    }
}
```

I might be biased, but I **much** prefer `htmeta`'s straightforwardness!

## More Information
Checkout the [repository](https://github.com/Diegovsky/htmeta). You can find examples and more documentation there!

Read the [details](./DETAILS.md) document for, well, details on what you can do with `htmeta`!.

[`KDL`]: kdl.dev

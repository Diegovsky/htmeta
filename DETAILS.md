This document details **precisely** which additions were made to `KDL` to make `htmeta`.

# Format
As [`KDL`] is already very similar to `HTML` semantically, `htmeta` only adds 2 things:
 - A way to differentiate true `text` content to be shown in `HTML`.
 - Variables to reduce repetition.

## Text arguments
If a node is only meant to have text content, you can specify content as the node's last argument!
```kdl
html {
    body {
        h1 "Title"
    }
}
```

Results in:

```html
<html>
    <body>
        <h1>Title</h1>
    </body>
</html>
```

Note that theses nodes can't have children! If you need to mix children with text, take a look at the next section.

## Text nodes
Text nodes are named `-` and they can only have one positional
argument, which is the text to be directly pasted into the resulting `HTML`.

Example:
```kdl
html {
    body {
        h1 "Title"
        p {
            - "Hi, I'm inline text!"
            em "I'm emphasized inline text."
            - "I'm post-emphasized inline text"
        }
        
    }
}
```

Results in:
```html
<html>
    <body>
        <h1>Title</h1>
        <p>
            Hi, I'm inline text!
            <em>I'm emphasized inline text.</em>
            I'm post-emphasized inline text
        </p>
    </body>
</html>
```

## Variables
If you ever used CSS-based frameworks like `TailwindCSS` or `Bootstrap`, you
know how tedious it is to type the same classes over and over again. Hence,
`htmeta` implements a simple variable mechanism that reduces duplication.

Example:
```kdl
html {
    head {
        meta charset="utf-8"
        // Includes tailwindcss
        script src="https://cdn.tailwindcss.com"
    }
    body {
        // creates a variable called `$btn_class`
        $btn_class "border-1 rounded-lg"

        // Value of `$btn_class` is reused inside these buttons:
        button class="$btn_class ml-4"
        bttton class="$btn_class mr-4"
    }
}
```

Results in:
```html
<html>
    <head>
        <meta charset="utf-8"/>
        <script src="https://cdn.tailwindcss.com"></script>
    </head>
    <body>
        <button class="border-1 rounded-lg ml-4"></button>
        <bttton class="border-1 rounded-lg mr-4"></bttton>
    </body>
</html>
```

[`KDL`]: https://kdl.dev/

This document details **precisely** which additions were made to `KDL` to make `htmeta`.

# Format
As [`KDL`] is already very similar to `HTML` semantically, `htmeta` only adds 2 things:
 - A way to differentiate true `text` content to be shown in `HTML`.
 - Variables to reduce repetition.

## Text nodes
Text nodes are creatively named `text` (or `content`) and they can only have one positional
argument, which is the text to be directly pasted into the resulting `HTML`.

Example:
```kdl
html {
    body {
        h1 {
            text "Title"
        }
    }
}
```

Results in:
```html
<html>
    <body>
        <h1>
            Title
        </h1>
    </body>
</html>
```

### Text attributes
As it soon gets verbose to open tags just to add a `text` node, a `text` (or `content`) attribute was added for quick, short content: 

```kdl
html {
    body {
        h1 text="Title"
        p text="I'm a paragraph!"
        button text="I'm a button!"
        div {
            content "You can still have text nodes if you want!"
        }
    }
}
```

Results in:
```html
<html>
    <body>
        <h1>Title</h1>
        <p>I'm a paragraph!</p>
        <button>I'm a button!</button>
        <div>
            You can still have text nodes if you want!
        </div>
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

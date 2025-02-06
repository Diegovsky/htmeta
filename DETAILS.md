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

## Raw Nodes
Raw nodes are named `_` and can be used to directly paste `HTML` without escaping text. Note that variables are still processed.
Its use is basically the same as the text node: Receives only one positional argument.

Example:
```kdl
html {
    head {
        script {
            _ "document.querySelector('body').innerHTML = `<h1>Dynamic Content!</h1>`"
        }
    }
    body {
        h1 "Boring Static Content"
    }
}
```

Results in:
```html
<html>
    <head>
        <script>
            document.querySelector('body').innerHTML = `<h1>Dynamic Content!</h1>`
        </script>
    </head>
    <body>
        <h1>Boring Static Content</h1>
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

# Templates
This crate also comes with a simple templating plugin that is automatically enabled in the CLI!

This plugin uses special nodes called meta-tags, which always start with an `@`, to differentiate them from normal nodes. They never show up in the generated `HTML` when recognized.
Which means that if a meta-tag shows up in your `HTML`, you probably did a typo.

Also, meta-tags are scoped to their parent, just like normal variables.

## Basics
Templates are defined by the `@template` meta-tag. You can specify the template's name both via a `name=` property, or as its first argument.

As you can see in the following example, templates work a bit like variables, except that they store whole node hierachies instead of strings:

Example:
```kdl
html {
    @template spacer {
        div class="invisible grow"
    }
    body class="flex flex-col min-h-screen w-full" {
        p "Text at the top of the page"
        @spacer
        p "Text at the bottom of the page"
    }
}
```

Results in:
```html
<html>
    <body class="flex flex-col min-h-screen w-full">
        <p>Text at the top of the page</p>
        <div class="invisible grow"></div>
        <p>Text at the bottom of the page</p>
    </body>
</html>
```

## Template parameters
Simple pasting is useful, but what if you wanted to write reusable components, or just reduce copy-pasting?

To solve this, template uses can be parametrized via `properties=` and `arguments`. Arguments turn into `$0`, `$1`, `$2` and so on according to their order.

The following example demonstrates a custom `p` template named `my-p`, which gets its content from a `$text` property.

Example:
```kdl
html {
    $padding "2"
    // Defines template `text`
    @template my-p {
        // Note how the template can access variables from its parent!
        p class="p-$padding rounded border-2 border-zinc-800" $text
    }
    body {
        h1 "Testing the templates"
        // Uses template `text`
        @my-p "Lorem ipsum dolor"
        div {
            h2 "Inside other nodes, too"
            // As properties get turned into variables inside the template,
            // you can override them like this
            @my-p padding="4" "Latin is super boring"
        }
    }
}
```

Results in:
```html
<html>
    <body>
        <h1>Testing the templates</h1>
        <p class="p-2 rounded border-2 border-zinc-800"></p>
        <div>
            <h2>Inside other nodes, too</h2>
            <p class="p-4 rounded border-2 border-zinc-800"></p>
        </div>
    </body>
</html>
```

## Template components



[`KDL`]: https://kdl.dev/

# neptungen
Yet another static Website Generator

# Why
Have you ever designed a static website for your club or one of your relatives but you don't want to maintain the contents for them? Well most static website generators are either dedicated to bloggers or they are too complicated to be used by non digital natives. 

Neptungen should be easy to use and minutes to set up even if you aren't an experienced web developer.

But probably the real reason for neptungen was the desire to learn programming in Rust. So over time the code will hopefully get more idiomatic.

# Features
- Completely written in Rust
- CMS based on CommonMark
- Builtin gallery generator
- Builtin FTP synchronization
- Customizable via liquid templates

# Getting started
Create a new root folder for your website
```bash
mkdir my_new_website
```
Each folders beneath that root folder represents a separate page of your website. The name of such a subfolder will used as a label in the navigation menu.
```bash
cd my_new_website
mkdir nav1
mkdir nav2
mkdir nav3
```

How can you add content to a page? Well, neptungen searches for markdown files and turns them into html which in turn is handed over to the page template. Markdown files must have the *.md extension.
```bash
cd nav1
touch index.md
...
```

Open and edit the markdown files with the markdown editor of your choice.
Each folder should only contain one markdown file plus the images you reference in your markdown file.

The final step is to generate the site. Therefore cd in the root directory and run neptungen as follows:
```bash
cd ../../my_new_website
/path/to/your/neptungen_executable/neptungen build
``` 

The generated output can typically be found in the `_output` directory.

# Galleries
Galleries are as simple as normal pages. Create a `gallery` sub directory within your normal page directory. Copy or symlink all relevant images into it. Finally invoke the generator again as already described before.

The sizes of the images and their thumbs can be controlled via the config.toml configuration file. 

# config.toml
Neptungen can be tweaked with the config.toml file. [TOML](https://github.com/toml-lang/toml) aims to be a minimal configuration file format that's easy to read due to obvious semantics. Neptungen offeres the following configuration options:


# Customize your website
You don't want to use the builtin website theme? Create the new file config.toml in the root folder of your project.

TODO: explain config parameters





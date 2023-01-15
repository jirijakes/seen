use webpage::HTML;

pub fn details(s: String) {
    let x = HTML::from_string(s, None).unwrap();

    println!("{:?}", x.title);
    println!("{:?}", x.description);
    println!("{:?}", x.language);
    println!("{:?}", x.meta);
    println!("{:?}", x.opengraph);
    println!("{:?}", x.schema_org);
}

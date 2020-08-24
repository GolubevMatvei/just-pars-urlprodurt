use clap::{App, crate_version};
use std::path::Path;
use std::fs::File;
use std::io::prelude::*;
use reqwest::Url;
use scraper::{Html, Selector};
use toml::Value;
use std::{thread, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error> > {

    //Списки - просто удобный способ получить аргументы, переданные программе
    let matches = App::new("just-pars-urlproduct")
        .author("Golubev Matvei, golubevmt@gmail.com")
        .version(crate_version!())
        .about("Консольная утелята для парсинра url продуктов с сайта ptatel.ru")
        //.arg("-_, --___=[___] '_____'")
        .arg("-o, --output=[DIR] 'Путь, куда сохранять json файл. Например: ./urllist.json'")
        .arg("<INPUT>'Путь до списка url ссылок на каталоги товаров'")
        .get_matches();

    //Получаем и проверяем директорию для сохранения файла
    let output = {
        let output_str = matches.value_of("output").unwrap_or("./");
        let output_path = Path::new(output_str);

        if !output_path.is_dir() {
            eprintln!("Ошибка: ожидается путь до каталога!");
            return Ok(())
        }

        output_path
    };

    //Читаем конфиги из config папки, из конфигов получаем ligin и oaswordd
    let (login, paswordd, sleep) = {

        let mut f = File::open("./config/config.toml")?;
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;

        let config = buf.parse::<Value>()?;

        let login_and_pasword = config.get("login_and_pasword").expect("Не найден раздел login_and_pasword");

        let login = login_and_pasword.get("login").expect("Не найден loggin").
            as_str().unwrap().to_string();
        let paswordd = login_and_pasword.get("pasword").expect("Не найден pasword").
            as_str().unwrap().to_string();

        println!("Login: {} Pasword: {}", login, paswordd);

        let sleep_v = config.get("parsset").expect("Не удалось найти раздел parsset")
            .get("sleep").expect("Не удалось найти sleep");

        let sleep = if sleep_v.is_float() {
            sleep_v.as_float().unwrap()
        } else if sleep_v.is_integer() {
            sleep_v.as_integer().unwrap() as f64
        } else { 0f64 };

        println!("Sleep {} sec", sleep);
            
        
        (login, paswordd, sleep)
    };

    //Собираем requwest client, отправляеи форму с login и paswordd для авторизации/подгрузки кэша на сайте piratel.ru
    let client = {
        let client = reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:78.0) Gecko/20100101 Firefox/78.0")
            .cookie_store(true)
            .gzip(true)
            .build()?;

        let params = [
            ("AUTH_FORM", "Y"),
            ("TYPE", "AUTH"),
            ("backurl", "/auth/"),
            ("USER_LOGIN", login.as_str()),
            ("USER_PASSWORD", paswordd.as_str()),
            ("Login", "Войти")];

        let html = client.post("https://pitatel.ru/auth/?login=yes")
            .form(&params)
            .send()?;

        let dom = Html::parse_document(html.text().unwrap().as_str());

        let selector = Selector::parse("div.col-xs-12:nth-child(2) > div:nth-child(1) > a:nth-child(2)").unwrap();
        
        match dom.select( &selector ).nth(0) {
            Some(element) => {
                let username = element.inner_html();
                if username == "Регистрация".to_string() {
                    eprintln!("Ошибка: не удалось авторезироватся, проверте верен ли логин и пароль!");
                    return Ok(())
                }
                println!("Username: {}", username);
            },
            None => panic!("Не удалось найти username поле на странице. Это странно, и вы не должны были увидеть это сообщение")
        }

        client
    };


    println!("\n\nСтарт!\n");

    //Получим путь до списка каталогов продуктов сайта pitatel.ru
    let path_to_catalog = {
        let input = matches.value_of_os("INPUT")
        .expect("Не удалось получить данные из INPUT. Это странно, и вы не должны были увидеть это сообщение");

        let path = Path::new(input);

        if !path.is_file() || path.extension().unwrap() != std::ffi::OsStr::new("txt") {
            eprintln!("Ошибка: <INPUT> ожидаеи файл с расширением .txt :(");
            return Ok(())
        }

        path
    };

    //Загружаеи url ссылки на каталоги товаров
    let catalogs = {
        let mut buf = String::new();
        File::open(path_to_catalog)?.read_to_string(&mut buf)?;
        buf      
    };

    let  product_urls = {

        let mut product_urls = Vec::new();
        
        //Обходим в цикле список сслок на каталоги товаров pitatel.ru
        for (i, catalog) in catalogs.split('\n').enumerate() {


                //Проверяем является ли строка URL адресом
            let url = match Url::parse(catalog) {
                Ok(url) => {
                    println!("Парсинг каталога: {}\n", url.as_str());
                    url
                },
                Err(_err) => {
                    eprintln!("Ошибка: строка {} не является URL ссылкой!", i+1);
                    continue
                }
            };

                //Загружае первую страницу, получаем чилсо страниц в каталоге
            let (dom ,count_pagess) = {
                let html = client.get(url.as_str()).send()?;

                if html.status() != reqwest::StatusCode::OK {
                    eprintln!("Ошибка: ответ от сервера с котдом {}!", html.status());
                    continue
                }

                    let dom = Html::parse_document(&html.text().unwrap());

                let selector = Selector::parse(".page-nav > li:nth-child(7) > a:nth-child(1)").unwrap();
                let count_pagess = dom.select(&selector).next().unwrap().inner_html();

                    (dom, count_pagess.parse::<u32>().unwrap())
            };

            println!("Число страниц: {}", count_pagess );
            println!("Стр. 1");
            //Загружаем все href ссылки с page=1
            let selector = Selector::parse("div.product__title > a:nth-child(1)").unwrap();

            let mut href_list = Vec::new();

            dom.select(&selector).for_each(  | val| {
                let href =  val.value().attr("href").unwrap();
                println!("  {}", href);
                href_list.push(href.to_string());

            });


            //Загружаем все href ссылки с page 2..=count_pagess
            for i_page in 2..=count_pagess {

                thread::sleep(Duration::from_secs_f64(sleep));
                
                let mut url_query = Url::parse(url.as_str()).unwrap();
                url_query.set_query(Some( format!("PAGEN_1={}", i_page).as_str() ));

                let html = client.get(url_query).send()?;
                let dom = Html::parse_document(&html.text().unwrap());

                println!("Стр. {} из {}", i_page, count_pagess);

                dom.select(&selector).for_each(  | val| {
                    let href =  val.value().attr("href").unwrap();
                    println!("  {}", href);               
                    href_list.push(href.to_string());
                });
            }


            //Конвертируем href ссылки в url
            href_list.iter().for_each(|href| {
                let mut product_url = Url::parse("https://pitatel.ru/").unwrap();
                product_url.set_path(href);

                product_urls.push(product_url);
            });
        };

        product_urls
    };

    //Преобразуем Url в Value::String
    let product_jstrings: Vec<_> = product_urls.iter()
        .map(|url| {
            serde_json::Value::String(url.to_string())
        })
        .collect();

    //Создаем из вектоара Value::String Value::Array   
    let product_value = serde_json::Value::Array(product_jstrings);

    //Создаем Value::Object
    let mut json_value = serde_json::Value::Object(serde_json::Map::new());
    json_value.as_object_mut().unwrap().insert("Товары".into(), product_value);

    //Сохраняем json файл в директорию output
    File::create(output.with_file_name("urlList.json"))?.write_all(json_value.to_string().as_bytes())?;    

    Ok(())
}
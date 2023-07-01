use std::{fs::File, path::PathBuf, time::Duration};

use anyhow::{Context as _, Result};
use oxhttp::{
    model::{Request, Response, ResponseBuilder, Status},
    Server,
};
use path_tree::PathTree;
use rusqlite::{params, Connection, Transaction};

use crate::models::{
    Character, CharacterStat, Curve, Pairs, Params, Part, PartType, Rarity, Stat, State, Weapon,
    WeaponCurve, Company,
};

static FAVICON_ANDROID_192: &[u8] = include_bytes!("../frontend/dist/android-chrome-192x192.png");
static FAVICON_ANDROID_512: &[u8] = include_bytes!("../frontend/dist/android-chrome-512x512.png");
static FAVICON_APPLE: &[u8] = include_bytes!("../frontend/dist/apple-touch-icon.png");
static FAVICON_16: &[u8] = include_bytes!("../frontend/dist/favicon-16x16.png");
static FAVICON_32: &[u8] = include_bytes!("../frontend/dist/favicon-32x32.png");
static FAVICON: &[u8] = include_bytes!("../frontend/dist/favicon.ico");
static WEB_MANIFEST: &[u8] = include_bytes!("../frontend/dist/site.webmanifest");

static INDEX: &str = include_str!("../frontend/dist/index.html");
static CSS: &str = include_str!("../frontend/dist/assets/index.css");
static JS: &str = include_str!("../frontend/dist/assets/index.js");

static LEVEL_MAX: u8 = 32;

static DATABASE_INIT_SQL: &str = "
    CREATE TABLE IF NOT EXISTS characters (name TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS template (key TEXT NOT NULL, type TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS stats (character TEXT NOT NULL, key TEXT NOT NULL, value INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS weapon_curves (name TEXT NOT NULL, type TEXT NOT NULL, a REAL NOT NULL, b REAL NOT NULL, c REAL NOT NULL, d REAL NOT NULL);
    CREATE TABLE IF NOT EXISTS weapon_parts (name TEXT NOT NULL, details TEXT NOT NULL, type TEXT NOT NULL, rarity TEXT NOT NULL, company TEXT NOT NULL);
";
static WEAPON_PARTS_SQL: &str = "
    INSERT INTO weapon_curves VALUES
        ('assault rifle', 'cubic', 0.25, 1, 0.25, 1),
        ('grenade launcher', 'cubic', 0.25, 1, 0.25, 1),
        ('pistol', 'cubic', 0.25, 1, 0.25, 1),
        ('rocket launcher', 'cubic', 0.25, 1, 0.25, 1),
        ('shotgun', 'cubic', 0.25, 1, 0.25, 1),
        ('sniper rifle', 'cubic', 0.25, 1, 0.25, 1),
        ('submachine gun', 'cubic', 0.25, 1, 0.25, 1);

    INSERT INTO weapon_parts VALUES
        -- technological
        ('lightweight', '', 'barrel', 'common', 'arksys'),
        ('hybrid', '', 'barrel', 'uncommon', 'arksys'),
        ('ansible', 'lore: wanna see me do it again?', 'barrel', 'rare', 'arksys'),
        ('ni-cad', '', 'body', 'common', 'arksys'),
        ('semiconductor', '', 'body', 'uncommon', 'arksys'),
        ('innovation', 'lore: science bitch!', 'body', 'rare', 'arksys'),
        ('<unnamed>', '', 'magazine', 'common', 'arksys'),
        ('<unnamed>', '', 'stock', 'common', 'arksys'),

        -- royalty/class (space greece)
        ('martial', '', 'barrel', 'common', 'dikarum'),
        ('noble', '', 'barrel', 'uncommon', 'dikarum'),
        ('bedazzled', '', 'barrel', 'rare', 'dikarum'),
        ('heir', 'lore: ...and soon it will be mine', 'body', 'common', 'dikarum'),
        ('aristocrat', '', 'body', 'uncommon', 'dikarum'),
        ('pony', 'lore: i want one!', 'body', 'rare', 'dikarum'),
        ('<unnamed>', '', 'magazine', 'common', 'dikarum'),
        ('<unnamed>', '', 'stock', 'common', 'dikarum'),

        -- science
        -- body: ration, plank
        ('ocular', '', 'barrel', 'common', 'pecora'),
        ('synthesized', 'lore: just like the real thing!', 'barrel', 'uncommon', 'pecora'),
        ('ionized', '', 'barrel', 'rare', 'pecora'),
        ('flicker', '', 'body', 'common', 'pecora'),
        ('railgun', 'lore: if it fits, it ships', 'body', 'uncommon', 'pecora'),
        ('inator', '', 'body', 'rare', 'pecora'),
        ('<unnamed>', '', 'magazine', 'common', 'pecora'),
        ('<unnamed>', '', 'stock', 'common', 'pecora'),

        -- religious/cult (nuns with guns)
        ('adamant', '', 'barrel', 'common', 'sisterhood'),
        ('sender', 'lore: hit like a sack of wet mice', 'barrel', 'uncommon', 'sisterhood'),
        ('blazing', '', 'barrel', 'rare', 'sisterhood'),
        ('lament', 'lore: hear you calling like a siren singing', 'body', 'common', 'sisterhood'),
        ('crutch', '', 'body', 'uncommon', 'sisterhood'),
        ('devote', 'lore: godspeed, black emperor', 'body', 'rare', 'sisterhood'),
        ('<unnamed>', '', 'magazine', 'common', 'sisterhood'),
        ('<unnamed>', '', 'stock', 'common', 'sisterhood'),

        -- space
        ('core', '', 'barrel', 'common', 'theia'),
        ('devoid', 'lore: dont be afraid of the end of the world', 'barrel', 'uncommon', 'theia'),
        ('lagrange', '', 'barrel', 'rare', 'theia'),
        ('tyche', '', 'body', 'common', 'theia'),
        ('cloud', 'lore: thats a big damn cloud', 'body', 'uncommon', 'theia'),
        ('three-body', '', 'body', 'rare', 'theia'),
        ('<unnamed>', '', 'magazine', 'common', 'theia'),
        ('<unnamed>', '', 'stock', 'common', 'theia'),

        -- geology/mining
        ('dusted', '', 'barrel', 'common', 'west_field'),
        ('catastrophic', 'lore: predestined to decay', 'barrel', 'uncommon', 'west_field'),
        ('hushing', '', 'barrel', 'rare', 'west_field'),
        ('reef', '', 'body', 'common', 'west_field'),
        ('placer', 'lore: in one, out the other', 'body', 'uncommon', 'west_field'),
        ('high-wall', '', 'body', 'rare', 'west_field'),
        ('<unnamed>', '', 'magazine', 'common', 'west_field'),
        ('<unnamed>', '', 'stock', 'common', 'west_field');
";

enum Route {
    Index,
    Css,
    Js,

    State,
    CharNew,
    CharRemove,
    CharStatIncrement,
    CharStatDecrement,
    CharStatToggle,
    StatNew,
    StatRemove,
    WeaponBuild,
    WeaponGenerate,
    WeaponPartInit,
    WeaponPartNew,
    WeaponPartRemove,

    FaviconAndroid192,
    FaviconAndroid512,
    FaviconApple,
    Favicon16,
    Favicon32,
    Favicon,
    WebManifest,
}

static DB: &str = "lumen.db";

#[rustfmt::skip]
fn main() -> anyhow::Result<()> {
    if !PathBuf::from(DB).exists() {
        File::create(PathBuf::from(DB))?;
    }

    {
        let conn = Connection::open(DB)?;

        conn.execute_batch(DATABASE_INIT_SQL)?;
    }

    let mut router: PathTree<Route> = PathTree::new();

    router.insert("/", Route::Index);
    router.insert("/assets/index.css", Route::Css);
    router.insert("/assets/index.js", Route::Js);

    router.insert("/api/state", Route::State);
    router.insert("/api/character/new", Route::CharNew);
    router.insert("/api/character/remove/:name", Route::CharRemove);
    router.insert("/api/character/increment/:name/:stat", Route::CharStatIncrement);
    router.insert("/api/character/decrement/:name/:stat", Route::CharStatDecrement);
    router.insert("/api/character/toggle/:name/:stat", Route::CharStatToggle);
    router.insert("/api/stat/new", Route::StatNew);
    router.insert("/api/stat/remove/:name", Route::StatRemove);
    router.insert("/api/weapon/build", Route::WeaponBuild);
    router.insert("/api/weapon/generate", Route::WeaponGenerate);
    router.insert("/api/weapon/part/init", Route::WeaponPartInit);
    router.insert("/api/weapon/part/new", Route::WeaponPartNew);
    router.insert("/api/weapon/part/remove/:name", Route::WeaponPartRemove);

    router.insert("/android-chrome-192x192.png", Route::FaviconAndroid192);
    router.insert("/android-chrome-512x512.png", Route::FaviconAndroid512);
    router.insert("/apple-touch-icon.png", Route::FaviconApple);
    router.insert("/favicon-16x16.png", Route::Favicon16);
    router.insert("/favicon-32x32.png", Route::Favicon32);
    router.insert("/favicon.ico", Route::Favicon);
    router.insert("/site.webmanifest", Route::WebManifest);

    let mut server = Server::new(move |request| {
        handle(&router, request)
    });

    server.set_global_timeout(Duration::from_secs(10));

    server.listen(("localhost", 8080))?;

    Ok(())
}

fn handle(router: &PathTree<Route>, request: &mut Request) -> Response {
    if let Some((route, path)) = router.find(request.url().path()) {
        let pairs = Pairs::new(request.url().query_pairs().collect::<Vec<_>>());
        let params = Params::new(path.params());

        let handler = match route {
            Route::Index => handlers::index,
            Route::Css => handlers::css,
            Route::Js => handlers::js,

            Route::State => handlers::state,
            Route::CharNew => handlers::char_new,
            Route::CharRemove => handlers::char_remove,
            Route::CharStatIncrement => handlers::char_stat_increment,
            Route::CharStatDecrement => handlers::char_stat_decrement,
            Route::CharStatToggle => handlers::char_stat_toggle,
            Route::StatNew => handlers::stat_new,
            Route::StatRemove => handlers::stat_remove,
            Route::WeaponBuild => handlers::weapon_build,
            Route::WeaponGenerate => handlers::weapon_generate,
            Route::WeaponPartInit => handlers::weapon_part_init,
            Route::WeaponPartNew => handlers::weapon_part_new,
            Route::WeaponPartRemove => handlers::weapon_part_remove,

            Route::FaviconAndroid192 => return Response::ok().with_body(FAVICON_ANDROID_192),
            Route::FaviconAndroid512 => return Response::ok().with_body(FAVICON_ANDROID_512),
            Route::FaviconApple => return Response::ok().with_body(FAVICON_APPLE),
            Route::Favicon16 => return Response::ok().with_body(FAVICON_16),
            Route::Favicon32 => return Response::ok().with_body(FAVICON_32),
            Route::Favicon => return Response::ok().with_body(FAVICON),
            Route::WebManifest => return Response::ok().with_body(WEB_MANIFEST),
        };

        return match handler(params, pairs) {
            Ok(res) => res,
            Err(err) => {
                Response::builder(Status::INTERNAL_SERVER_ERROR).with_body(format!("{:#?}", err))
            }
        };
    }

    Response::builder(Status::NOT_FOUND).build()
}
trait ResponseExt {
    fn ok() -> ResponseBuilder;
}

impl ResponseExt for Response {
    fn ok() -> ResponseBuilder {
        Response::builder(Status::OK)
    }
}

trait AsConn {
    fn as_conn(&self) -> &Connection;
}

impl<'conn> AsConn for &'conn Connection {
    fn as_conn(&self) -> &Connection {
        self
    }
}

impl<'conn> AsConn for &'conn Transaction<'conn> {
    fn as_conn(&self) -> &Connection {
        self
    }
}

struct Db;

impl Db {
    fn state<C: AsConn>(conn: C, weapon: Option<Weapon>) -> Result<State> {
        let conn = conn.as_conn();

        Ok(State {
            stats: Self::template(conn)?,
            characters: Self::characters(conn)?,
            parts: Self::parts(conn)?,
            weapon: weapon.map(Weapon::display),
        })
    }

    fn add_character<C: AsConn, A: AsRef<str>>(conn: C, name: A) -> Result<()> {
        let conn = conn.as_conn();

        conn.execute("INSERT INTO characters VALUES (?)", [name.as_ref()])?;

        let mut stat_stmt = conn.prepare("INSERT INTO stats VALUES (?, ?, ?)")?;

        let mut template_stmt = conn.prepare("SELECT key FROM template")?;
        let mut template_rows = template_stmt.query(params![])?;
        while let Some(template_row) = template_rows.next()? {
            stat_stmt.execute(params![
                name.as_ref(),
                &template_row.get::<_, String>(0)?,
                0
            ])?;
        }

        Ok(())
    }

    fn remove_character<C: AsConn, A: AsRef<str>>(conn: C, name: A) -> Result<()> {
        let conn = conn.as_conn();

        conn.execute(
            "DELETE FROM characters WHERE name = ?",
            params![name.as_ref()],
        )?;
        conn.execute(
            "DELETE FROM stats WHERE character = ?",
            params![name.as_ref()],
        )?;

        Ok(())
    }

    fn add_stat<C: AsConn, A: AsRef<str>>(conn: C, name: A, typ: A) -> Result<()> {
        let conn = conn.as_conn();

        conn.execute(
            "INSERT INTO template VALUES (?, ?)",
            [name.as_ref(), typ.as_ref()],
        )?;

        let mut stat_stmt = conn.prepare("INSERT INTO stats VALUES (?, ?, ?)")?;

        let mut character_stmt = conn.prepare("SELECT name FROM characters")?;
        let mut character_rows = character_stmt.query(params![])?;
        while let Some(character_row) = character_rows.next()? {
            stat_stmt.execute(params![
                &character_row.get::<_, String>(0)?,
                name.as_ref(),
                0
            ])?;
        }

        Ok(())
    }

    fn remove_stat<C: AsConn, A: AsRef<str>>(conn: C, name: A) -> Result<()> {
        let conn = conn.as_conn();

        conn.execute("DELETE FROM template WHERE key = ?", params![name.as_ref()])?;
        conn.execute("DELETE FROM stats WHERE key = ?", params![name.as_ref()])?;

        Ok(())
    }

    fn increment_stat<C: AsConn, A: AsRef<str>>(conn: C, name: A, stat: A) -> Result<()> {
        let conn = conn.as_conn();

        conn.execute(
            "UPDATE stats SET value = value + 1 WHERE character = ? AND key = ?",
            params![name.as_ref(), stat.as_ref()],
        )?;

        Ok(())
    }

    fn decrement_stat<C: AsConn, A: AsRef<str>>(conn: C, name: A, stat: A) -> Result<()> {
        let conn = conn.as_conn();

        conn.execute(
            "UPDATE stats SET value = value - 1 WHERE character = ? AND key = ?",
            params![name.as_ref(), stat.as_ref()],
        )?;

        Ok(())
    }

    fn toggle_stat<C: AsConn, A: AsRef<str>>(conn: C, name: A, stat: A) -> Result<()> {
        let conn = conn.as_conn();

        conn.execute("UPDATE stats SET value = CASE value WHEN 0 THEN 1 ELSE 0 END WHERE character = ? AND key = ?", params![name.as_ref(), stat.as_ref()])?;

        Ok(())
    }

    fn add_weapon_part<C: AsConn, A: AsRef<str>>(
        conn: C,
        name: A,
        details: A,
        part: PartType,
        rarity: Rarity,
        company: Company,
    ) -> Result<()> {
        let conn = conn.as_conn();

        conn.execute(
            "INSERT INTO weapon_parts VALUES (?, ?, ?, ?, ?)",
            params![name.as_ref(), details.as_ref(), part, rarity, company],
        )?;

        Ok(())
    }

    fn remove_weapon_part<C: AsConn, A: AsRef<str>>(conn: C, name: A) -> Result<()> {
        let conn = conn.as_conn();

        conn.execute(
            "DELETE FROM weapon_parts WHERE name = ?",
            params![name.as_ref()],
        )?;

        Ok(())
    }

    fn template<C: AsConn>(conn: C) -> Result<Vec<Stat>> {
        let conn = conn.as_conn();

        let mut template_stmt = conn.prepare("SELECT key, type FROM template")?;

        let template = template_stmt
            .query_map([], |row| {
                Ok(Stat {
                    name: row.get(0)?,
                    typ: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>();

        template.context("failed to collect template stats")
    }

    fn characters<C: AsConn>(conn: C) -> Result<Vec<Character>> {
        let conn = conn.as_conn();

        let mut characters_stmt = conn.prepare("SELECT name FROM characters")?;
        let mut stats_stmt = conn.prepare("SELECT s.key, (SELECT t.type FROM template t WHERE t.key = s.key) as type, s.value FROM stats s WHERE s.character = ?")?;

        let characters = characters_stmt
            .query_map([], |row| {
                let name = row.get(0)?;

                let stats = stats_stmt
                    .query_map([&name], |row| {
                        Ok(CharacterStat {
                            name: row.get(0)?,
                            typ: row.get(1)?,
                            value: row.get(2)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Character { name, stats })
            })?
            .collect::<Result<Vec<_>, _>>();

        characters.context("failed to collect characters")
    }

    fn curves<C: AsConn>(conn: C) -> Result<Vec<WeaponCurve>> {
        let conn = conn.as_conn();

        let mut parts_stmt = conn.prepare("SELECT name, type, a, b, c, d FROM weapon_curves")?;

        let parts = parts_stmt
            .query_map([], |row| {
                Ok(WeaponCurve {
                    typ: row.get(0)?,
                    curve: match row.get::<_, String>(1)?.as_str() {
                        "cubic" => Curve::cubic(row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?),
                        "linear" => Curve::linear(row.get(2)?, row.get(3)?),
                        "quadratic" => Curve::quadratic(row.get(2)?, row.get(3)?, row.get(4)?),
                        _ => panic!(),
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>();

        parts.context("failed to collect weapon curves")
    }

    fn parts<C: AsConn>(conn: C) -> Result<Vec<Part>> {
        let conn = conn.as_conn();

        let mut parts_stmt =
            conn.prepare("SELECT name, details, type, rarity, company FROM weapon_parts")?;

        let parts = parts_stmt
            .query_map([], |row| {
                Ok(Part {
                    name: row.get(0)?,
                    details: row.get(1)?,
                    typ: row.get(2)?,
                    rarity: row.get(3)?,
                    company: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>();

        parts.context("failed to collect weapon parts")
    }
}

mod handlers {
    use anyhow::{Context as _, Result};
    use oxhttp::model::{Response, Status};
    use rusqlite::Connection;

    use crate::{
        models::{Id, Pairs, Params, PartType, Rarity, Weapon, Company},
        Db, CSS, DB, INDEX, JS, WEAPON_PARTS_SQL,
    };

    fn json<S: serde::Serialize>(s: S) -> Result<Response> {
        let body = serde_json::to_string(&s)?;

        Ok(Response::builder(Status::OK)
            .with_header("Content-Type", "application/json")?
            .with_body(body))
    }

    pub fn index(_params: Params<'_>, _pairs: Pairs<'_>) -> Result<Response> {
        Ok(Response::builder(Status::OK)
            .with_header("Content-Type", "text/html; charset=utf-8")?
            .with_body(INDEX))
    }

    pub fn css(_params: Params<'_>, _pairs: Pairs<'_>) -> Result<Response> {
        Ok(Response::builder(Status::OK)
            .with_header("Content-Type", "text/css; charset=utf-8")?
            .with_body(CSS))
    }

    pub fn js(_params: Params<'_>, _pairs: Pairs<'_>) -> Result<Response> {
        Ok(Response::builder(Status::OK)
            .with_header("Content-Type", "text/javascript; charset=utf-8")?
            .with_body(JS))
    }

    pub fn state(_: Params<'_>, _pairs: Pairs<'_>) -> Result<Response> {
        let conn = Connection::open(DB)?;

        json(Db::state(&conn, None)?)
    }

    pub fn char_new(_: Params<'_>, pairs: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        let value = pairs.find("name")?;
        Db::add_character(&trans, value)?;

        trans.commit()?;

        json(Db::state(&conn, None)?)
    }

    pub fn char_remove(params: Params<'_>, _: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        let name = params.find("name")?;
        Db::remove_character(&trans, name)?;

        trans.commit()?;

        json(Db::state(&conn, None)?)
    }

    pub fn char_stat_increment(params: Params<'_>, _: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        let name = params.find("name")?;
        let stat = params.find("stat")?;
        Db::increment_stat(&trans, name, stat)?;

        trans.commit()?;

        json(Db::state(&conn, None)?)
    }

    pub fn char_stat_decrement(params: Params<'_>, _: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        let name = params.find("name")?;
        let stat = params.find("stat")?;
        Db::decrement_stat(&trans, name, stat)?;

        trans.commit()?;

        json(Db::state(&conn, None)?)
    }

    pub fn char_stat_toggle(params: Params<'_>, _: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        let name = params.find("name")?;
        let stat = params.find("stat")?;
        Db::toggle_stat(&trans, name, stat)?;

        trans.commit()?;

        json(Db::state(&conn, None)?)
    }

    pub fn stat_new(_: Params<'_>, pairs: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        let name = pairs.find("name")?;
        let typ = pairs.find("type")?;
        Db::add_stat(&trans, name, typ)?;

        trans.commit()?;

        json(Db::state(&conn, None)?)
    }

    pub fn stat_remove(params: Params<'_>, _: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        let name = params.find("name")?;
        Db::remove_stat(&trans, name)?;

        trans.commit()?;

        json(Db::state(&conn, None)?)
    }

    pub fn weapon_build(_params: Params<'_>, pairs: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        let id = Id::try_from(pairs.find("id")?)?;

        if !id.check() {
            return Ok(Response::builder(Status::BAD_REQUEST)
                .with_header("Location", "/")?
                .build());
        }

        let parts = Db::parts(&trans)?;
        let curves = Db::curves(&trans)?;

        let weapon = Weapon::from_id(&parts, &curves, id)
            .context("failed to generate weapon, missing part")?;

        trans.commit()?;

        json(Db::state(&conn, Some(weapon))?)
    }

    pub fn weapon_generate(_params: Params<'_>, pairs: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        let level = pairs.find("level")?.parse::<u8>()?;

        let parts = Db::parts(&trans)?;
        let curves = Db::curves(&trans)?;

        let weapon = Weapon::generate(&parts, &curves, level)
            .context("failed to generate weapon, missing part")?;

        trans.commit()?;

        json(Db::state(&conn, Some(weapon))?)
    }

    pub fn weapon_part_init(_: Params<'_>, _: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        trans.execute_batch(WEAPON_PARTS_SQL)?;

        trans.commit()?;

        json(Db::state(&conn, None)?)
    }

    pub fn weapon_part_new(_: Params<'_>, pairs: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        let name = pairs.find("name")?;
        let details = pairs.find("details")?;

        let part = PartType::try_from(pairs.find("part")?)?;
        let rarity = Rarity::try_from(pairs.find("rarity")?)?;
        let company = Company::try_from(pairs.find("company")?)?;

        Db::add_weapon_part(&trans, name, details, part, rarity, company)?;

        trans.commit()?;

        json(Db::state(&conn, None)?)
    }

    pub fn weapon_part_remove(params: Params<'_>, _: Pairs<'_>) -> Result<Response> {
        let mut conn = Connection::open(DB)?;
        let trans = conn.transaction()?;

        let name = params.find("name")?;
        Db::remove_weapon_part(&trans, name)?;

        trans.commit()?;

        json(Db::state(&conn, None)?)
    }
}

mod models {
    use std::{borrow::Cow, cmp::Ordering, fmt, ops};

    use anyhow::{Context as _, Result};
    use rand::{prelude::*, Rng};
    use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

    use crate::{utils, LEVEL_MAX};

    pub struct Pairs<'p>(Vec<(Cow<'p, str>, Cow<'p, str>)>);

    impl<'p> Pairs<'p> {
        pub fn new(pairs: Vec<(Cow<'p, str>, Cow<'p, str>)>) -> Self {
            Self(pairs)
        }

        pub fn find(&'p self, name: &str) -> Result<&'p str> {
            self.0
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.as_ref())
                .with_context(|| format!("missing `{}` url parameter", name))
        }
    }

    pub struct Params<'p>(Vec<(&'p str, &'p str)>);

    impl<'p> Params<'p> {
        pub fn new(pairs: Vec<(&'p str, &'p str)>) -> Self {
            Self(pairs)
        }

        pub fn find(&self, name: &str) -> Result<&'p str> {
            self.0
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| *v)
                .with_context(|| format!("missing `{}` path parameter", name))
        }
    }

    #[derive(Clone, Copy, serde::Serialize)]
    pub struct Id(#[serde(with = "id_serde")] [u8; 8]);

    impl Id {
        #[allow(dead_code)]
        pub fn new(id: u64) -> Self {
            Self(id.to_be_bytes())
        }

        pub fn from(level: u8, typ: u8, body: u8, barrel: u8, magazine: u8, stock: u8) -> Self {
            let parity = level ^ typ ^ body ^ barrel ^ magazine ^ stock;

            Self([level, typ, body, barrel, magazine, stock, 0, parity])
        }

        pub fn check(&self) -> bool {
            let [level, typ, body, barrel, magazine, stock, _, truth] = self.0;

            let result = level ^ typ ^ body ^ barrel ^ magazine ^ stock;

            truth == result
        }

        #[inline]
        pub fn level(&self) -> u8 {
            self.0[0]
        }

        #[inline]
        pub fn typ(&self) -> u8 {
            self.0[1]
        }
    }

    impl Id {
        #[inline]
        pub fn body(&self) -> u8 {
            self.0[2]
        }

        #[inline]
        pub fn barrel(&self) -> u8 {
            self.0[3]
        }

        #[inline]
        pub fn magazine(&self) -> u8 {
            self.0[4]
        }

        #[inline]
        pub fn stock(&self) -> u8 {
            self.0[5]
        }
    }

    impl TryFrom<&str> for Id {
        type Error = std::num::ParseIntError;

        fn try_from(id: &str) -> Result<Self, Self::Error> {
            u64::from_str_radix(id, 16).map(|id| Self(id.to_be_bytes()))
        }
    }

    impl std::fmt::Display for Id {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{:0>16x?}", u64::from_be_bytes(self.0))
        }
    }

    mod id_serde {
        use serde::Serializer;

        #[inline]
        pub fn serialize<S>(id: &[u8; 8], s: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            s.serialize_u64(u64::from_be_bytes(*id))
        }
    }

    #[derive(serde::Serialize)]
    pub struct State {
        pub stats: Vec<Stat>,
        pub characters: Vec<Character>,
        pub parts: Vec<Part>,
        pub weapon: Option<WeaponDisplay>,
    }

    #[derive(serde::Serialize)]
    pub struct Character {
        pub name: String,
        pub stats: Vec<CharacterStat>,
    }

    #[derive(serde::Serialize)]
    pub struct CharacterStat {
        pub name: String,
        #[serde(rename = "type")]
        pub typ: StatType,
        pub value: i32,
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Company {
        Arksys,
        Dikarum,
        Pecora,
        Sisterhood,
        Theia,
        WestField,
    }

    impl Company {
        fn to_lower_name(self) -> &'static str {
            match self {
                Company::Arksys => "arksys",
                Company::Dikarum => "dikarum",
                Company::Pecora => "pecora",
                Company::Sisterhood => "sisterhood",
                Company::Theia => "theia",
                Company::WestField => "west_field",
            }
        }
    }

    impl TryFrom<&str> for Company {
        type Error = FromSqlError;

        fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
            match value {
                "arksys" => Ok(Company::Arksys),
                "dikarum" => Ok(Company::Dikarum),
                "pecora" => Ok(Company::Pecora),
                "sisterhood" => Ok(Company::Sisterhood),
                "theia" => Ok(Company::Theia),
                "west_field" => Ok(Company::WestField),
                _ => Err(FromSqlError::InvalidType),
            }
        }
    }

    impl fmt::Display for Company {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Company::Arksys => write!(f, "Arksys Inc"),
                Company::Dikarum => write!(f, "Dikarum & Sons"),
                Company::Pecora => write!(f, "Pecora Group"),
                Company::Sisterhood => write!(f, "Sisterhood of Blight"),
                Company::Theia => write!(f, "Theia Manufacturing"),
                Company::WestField => write!(f, "West Field Mining Munitions"),
            }
        }
    }

    impl FromSql for Company {
        fn column_result(value: ValueRef) -> FromSqlResult<Self> {
            String::column_result(value).and_then(|s| Company::try_from(s.as_str()))
        }
    }

    impl ToSql for Company {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput> {
            Ok(self.to_lower_name().into())
        }
    }

    #[derive(Clone, serde::Serialize)]
    pub struct Part {
        pub name: String,
        pub details: String,
        #[serde(rename = "type")]
        pub typ: PartType,
        pub rarity: Rarity,
        pub company: Company,
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum PartType {
        Body,
        Barrel,
        Magazine,
        Stock,
    }

    impl TryFrom<&str> for PartType {
        type Error = FromSqlError;

        fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
            match value {
                "body" => Ok(PartType::Body),
                "barrel" => Ok(PartType::Barrel),
                "magazine" => Ok(PartType::Magazine),
                "stock" => Ok(PartType::Stock),
                _ => Err(FromSqlError::InvalidType),
            }
        }
    }

    impl fmt::Display for PartType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                PartType::Body => write!(f, "body"),
                PartType::Barrel => write!(f, "barrel"),
                PartType::Magazine => write!(f, "magazine"),
                PartType::Stock => write!(f, "stock"),
            }
        }
    }

    impl FromSql for PartType {
        fn column_result(value: ValueRef) -> FromSqlResult<Self> {
            String::column_result(value).and_then(|s| PartType::try_from(s.as_str()))
        }
    }

    impl ToSql for PartType {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput> {
            match self {
                PartType::Body => Ok("body".into()),
                PartType::Barrel => Ok("barrel".into()),
                PartType::Magazine => Ok("magazine".into()),
                PartType::Stock => Ok("stock".into()),
            }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum Rarity {
        Common,
        Uncommon,
        Rare,
        Epic,
        Legendary,
        Unique,
    }

    impl TryFrom<&str> for Rarity {
        type Error = FromSqlError;

        fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
            match value {
                "common" => Ok(Rarity::Common),
                "uncommon" => Ok(Rarity::Uncommon),
                "rare" => Ok(Rarity::Rare),
                "epic" => Ok(Rarity::Epic),
                "legendary" => Ok(Rarity::Legendary),
                "unique" => Ok(Rarity::Unique),
                _ => Err(FromSqlError::InvalidType),
            }
        }
    }

    impl fmt::Display for Rarity {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Rarity::Common => write!(f, "common"),
                Rarity::Uncommon => write!(f, "uncommon"),
                Rarity::Rare => write!(f, "rare"),
                Rarity::Epic => write!(f, "epic"),
                Rarity::Legendary => write!(f, "legendary"),
                Rarity::Unique => write!(f, "unique"),
            }
        }
    }

    impl ops::BitOr for Rarity {
        type Output = Self;

        fn bitor(self, rhs: Self) -> Self::Output {
            match self.cmp(&rhs) {
                Ordering::Less => rhs,
                Ordering::Equal => self,
                Ordering::Greater => self,
            }
        }
    }

    impl FromSql for Rarity {
        fn column_result(value: ValueRef) -> FromSqlResult<Self> {
            String::column_result(value).and_then(|s| Rarity::try_from(s.as_str()))
        }
    }

    impl ToSql for Rarity {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput> {
            match self {
                Rarity::Common => Ok("common".into()),
                Rarity::Uncommon => Ok("uncommon".into()),
                Rarity::Rare => Ok("rare".into()),
                Rarity::Epic => Ok("epic".into()),
                Rarity::Legendary => Ok("legendary".into()),
                Rarity::Unique => Ok("unique".into()),
            }
        }
    }

    #[derive(serde::Serialize)]
    pub struct WeaponDisplay {
        pub level: u8,
        pub id: String,
        pub name: String,
        pub rarity: Rarity,
        #[serde(rename = "type")]
        pub typ: WeaponType,
        pub company: Company,
        pub barrel: Part,
        pub body: Part,
        pub magazine: Part,
        pub stock: Part,
        pub range: String,
        pub damage: String,
        pub details: Vec<String>,
    }

    #[derive(serde::Serialize)]
    pub struct Weapon {
        pub level: u8,
        pub id: Id,
        pub rarity: Rarity,
        #[serde(rename = "type")]
        pub typ: WeaponType,
        pub company: Company,
        pub barrel: Part,
        pub body: Part,
        pub magazine: Part,
        pub stock: Part,
        pub damage: f32,
    }

    impl Weapon {
        pub fn generate(parts: &[Part], curves: &[WeaponCurve], level: u8) -> Result<Self> {
            use PartType::*;

            let mut rng = rand::thread_rng();

            let typ = match rng.gen_range(0..10) {
                0 | 1 => WeaponType::Pistol,
                2 | 3 => WeaponType::Submachine,
                4 | 5 => WeaponType::Shotgun,
                6 | 7 => WeaponType::Assault,
                8 => WeaponType::Grenade,
                9 => WeaponType::Sniper,
                10 => WeaponType::Rocket,
                _ => unreachable!(),
            };

            let curve = curves.iter().find(|c| c.typ == typ).context("Missing weapon curve")?;

            let (barrel_index, barrel) = Self::generate_part(parts, &mut rng, level, Barrel).context("Missing barrel")?;
            let (body_index, body) = Self::generate_part(parts, &mut rng, level, Body).context("Missing body")?;
            let (magazine_index, magazine) = Self::generate_part(parts, &mut rng, level, Magazine).context("Missing magazine")?;
            let (stock_index, stock) = Self::generate_part(parts, &mut rng, level, Stock).context("Missing stock")?;

            let rarity = body.rarity | barrel.rarity | magazine.rarity | stock.rarity;

            let id = Id::from(
                level,
                typ.index(),
                body_index as u8,
                barrel_index as u8,
                magazine_index as u8,
                stock_index as u8,
            );

            let damage = curve.curve.evaluate(utils::rescale(
                level as f32,
                0.0..(LEVEL_MAX as f32),
                0.0..1.0,
            ));

            Ok(Self {
                level,
                id,
                rarity,
                typ,
                company: body.company,
                barrel,
                body,
                magazine,
                stock,
                damage,
            })
        }

        fn generate_part<R: Rng>(
            parts: &[Part],
            rng: &mut R,
            level: u8,
            typ: PartType,
        ) -> Option<(usize, Part)> {
            let filtered = parts
                .iter()
                .enumerate()
                .filter(|(_, p)| p.typ == typ)
                .filter(|(_, p)| Self::filter_rarity(p, level));

            filtered.choose(rng).map(|(id, part)| (id, part.clone()))
        }

        fn filter_rarity(part: &Part, level: u8) -> bool {
            use Rarity::*;

            if level >= 20 {
                true
            } else if level >= 12 {
                matches!(part.rarity, Common | Uncommon | Rare | Epic)
            } else if level >= 8 {
                matches!(part.rarity, Common | Uncommon | Rare)
            } else {
                matches!(part.rarity, Common | Uncommon)
            }
        }

        pub fn from_id(parts: &[Part], curves: &[WeaponCurve], id: Id) -> Option<Self> {
            let level = id.level();
            let body = parts[id.body() as usize].clone();
            let barrel = parts[id.barrel() as usize].clone();
            let magazine = parts[id.magazine() as usize].clone();
            let stock = parts[id.stock() as usize].clone();

            let rarity = body.rarity | barrel.rarity | magazine.rarity | stock.rarity;

            let typ = WeaponType::from_index(id.typ())?;

            let curve = curves.iter().find(|c| c.typ == typ)?;

            let damage = curve.curve.evaluate(utils::rescale(
                level as f32,
                0.0..(LEVEL_MAX as f32),
                0.0..1.0,
            ));

            Some(Self {
                level,
                id,
                rarity,
                typ,
                company: body.company,
                barrel,
                body,
                magazine,
                stock,
                damage,
            })
        }

        pub fn name(&self) -> String {
            format!("{} {}", self.barrel.name, self.body.name)
        }

        pub fn damage(&self) -> String {
            let base = match self.typ {
                WeaponType::Assault => 1.0,
                WeaponType::Grenade => 1.0,
                WeaponType::Pistol => 1.0,
                WeaponType::Rocket => 1.0,
                WeaponType::Shotgun => 1.0,
                WeaponType::Sniper => 1.0,
                WeaponType::Submachine => 1.0,
            };

            (base + (base * self.damage).round()).to_string()
        }

        pub fn range(&self) -> &str {
            match self.typ {
                WeaponType::Assault => "mid-far",
                WeaponType::Grenade => "mid",
                WeaponType::Pistol => "close-near",
                WeaponType::Rocket => "mid-far",
                WeaponType::Shotgun => "mid",
                WeaponType::Sniper => "far",
                WeaponType::Submachine => "close-mid",
            }
        }

        pub fn details(&self) -> WeaponDetailsIter<'_> {
            WeaponDetailsIter {
                weapon: self,
                index: 0,
            }
        }

        pub fn display(self) -> WeaponDisplay {
            WeaponDisplay {
                level: self.level,
                id: self.id.to_string(),
                name: self.name(),
                rarity: self.rarity,
                typ: self.typ,
                company: self.company,
                barrel: self.barrel.clone(),
                body: self.body.clone(),
                magazine: self.magazine.clone(),
                stock: self.stock.clone(),
                range: self.range().to_string(),
                damage: self.damage(),
                details: self.details().collect(),
            }
        }
    }

    pub struct WeaponDetailsIter<'w> {
        weapon: &'w Weapon,
        index: u8,
    }

    impl<'w> Iterator for WeaponDetailsIter<'w> {
        type Item = String;

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                match self.index {
                    0 => {
                        if !self.weapon.body.details.is_empty() {
                            self.index += 1;

                            return Some(self.weapon.body.details.clone());
                        }
                    }
                    1 => {
                        if !self.weapon.barrel.details.is_empty() {
                            self.index += 1;

                            return Some(self.weapon.barrel.details.clone());
                        }
                    }
                    2 => {
                        if !self.weapon.magazine.details.is_empty() {
                            self.index += 1;

                            return Some(self.weapon.magazine.details.clone());
                        }
                    }
                    3 => {
                        if !self.weapon.stock.details.is_empty() {
                            self.index += 1;

                            return Some(self.weapon.stock.details.clone());
                        }
                    }
                    _ => return None,
                }

                self.index += 1;
            }
        }
    }

    pub struct WeaponCurve {
        pub typ: WeaponType,
        pub curve: Curve,
    }

    #[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum WeaponType {
        Assault,
        Grenade,
        Pistol,
        Rocket,
        Shotgun,
        Sniper,
        Submachine,
    }

    impl WeaponType {
        fn index(&self) -> u8 {
            match self {
                WeaponType::Assault => 0,
                WeaponType::Grenade => 1,
                WeaponType::Pistol => 2,
                WeaponType::Rocket => 3,
                WeaponType::Shotgun => 4,
                WeaponType::Sniper => 5,
                WeaponType::Submachine => 6,
            }
        }

        fn from_index(index: u8) -> Option<Self> {
            match index {
                0 => Some(WeaponType::Assault),
                1 => Some(WeaponType::Grenade),
                2 => Some(WeaponType::Pistol),
                3 => Some(WeaponType::Rocket),
                4 => Some(WeaponType::Shotgun),
                5 => Some(WeaponType::Sniper),
                6 => Some(WeaponType::Submachine),
                _ => None,
            }
        }
    }

    impl fmt::Display for WeaponType {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                WeaponType::Assault => write!(f, "assault rifle"),
                WeaponType::Grenade => write!(f, "grenade launcher"),
                WeaponType::Pistol => write!(f, "pistol"),
                WeaponType::Rocket => write!(f, "rocket launcher"),
                WeaponType::Shotgun => write!(f, "shotgun"),
                WeaponType::Sniper => write!(f, "sniper rifle"),
                WeaponType::Submachine => write!(f, "submachine gun"),
            }
        }
    }

    impl TryFrom<&str> for WeaponType {
        type Error = FromSqlError;

        fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
            match value {
                "assault rifle" => Ok(WeaponType::Assault),
                "grenade launcher" => Ok(WeaponType::Grenade),
                "pistol" => Ok(WeaponType::Pistol),
                "rocket launcher" => Ok(WeaponType::Rocket),
                "shotgun" => Ok(WeaponType::Shotgun),
                "sniper rifle" => Ok(WeaponType::Sniper),
                "submachine gun" => Ok(WeaponType::Submachine),
                _ => Err(FromSqlError::InvalidType),
            }
        }
    }

    impl FromSql for WeaponType {
        fn column_result(value: ValueRef) -> FromSqlResult<Self> {
            String::column_result(value).and_then(|s| WeaponType::try_from(s.as_str()))
        }
    }

    impl ToSql for WeaponType {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput> {
            Ok(self.to_string().into())
        }
    }

    #[derive(serde::Serialize)]
    pub struct Stat {
        pub name: String,
        #[serde(rename = "type")]
        pub typ: StatType,
    }

    #[derive(serde::Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum StatType {
        Number,
        Boolean,
    }

    impl FromSql for StatType {
        fn column_result(value: ValueRef) -> FromSqlResult<Self> {
            String::column_result(value).and_then(|s| match s.as_str() {
                "number" => Ok(StatType::Number),
                "boolean" => Ok(StatType::Boolean),
                _ => Err(FromSqlError::InvalidType),
            })
        }
    }

    impl ToSql for StatType {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput> {
            match self {
                StatType::Number => Ok("number".into()),
                StatType::Boolean => Ok("boolean".into()),
            }
        }
    }

    #[derive(Clone, Copy)]
    pub enum Curve {
        Linear(Linear),
        Quadratic(Quadratic),
        Cubic(Cubic),
    }

    impl Curve {
        #[inline]
        pub const fn linear(a: f32, b: f32) -> Self {
            Self::Linear(Linear::new(a, b))
        }

        #[inline]
        pub const fn quadratic(a: f32, b: f32, c: f32) -> Self {
            Self::Quadratic(Quadratic::new(a, b, c))
        }

        #[inline]
        pub const fn cubic(a: f32, b: f32, c: f32, d: f32) -> Self {
            Self::Cubic(Cubic::new(a, b, c, d))
        }

        #[inline]
        #[track_caller]
        pub fn evaluate(&self, t: f32) -> f32 {
            debug_assert!((0.0..=1.0).contains(&t), "value: {}", t);
            match self {
                Curve::Linear(curve) => curve.evaluate(t),
                Curve::Quadratic(curve) => curve.evaluate(t),
                Curve::Cubic(curve) => curve.evaluate(t),
            }
        }
    }

    /// A linear Bézier curve.
    #[derive(Clone, Copy)]
    pub struct Linear {
        a: f32,
        b: f32,
    }

    /// A quadratic Bézier curve.
    #[derive(Clone, Copy)]
    pub struct Quadratic {
        a: f32,
        b: f32,
        c: f32,
    }

    /// A cubic Bézier curve.
    #[derive(Clone, Copy)]
    pub struct Cubic {
        a: f32,
        b: f32,
        c: f32,
        d: f32,
    }

    impl Linear {
        /// Create a curve.
        #[inline]
        const fn new(a: f32, b: f32) -> Self {
            Self { a, b }
        }

        #[inline]
        fn evaluate(&self, t: f32) -> f32 {
            (1.0 - t) * self.a + t * self.b
        }
    }

    impl Quadratic {
        /// Create a curve.
        #[inline]
        const fn new(a: f32, b: f32, c: f32) -> Self {
            Self { a, b, c }
        }

        #[inline]
        fn evaluate(&self, t: f32) -> f32 {
            let c = 1.0 - t;
            c * c * self.a + 2.0 * c * t * self.b + t * t * self.c
        }
    }

    impl Cubic {
        /// Create a curve.
        #[inline]
        const fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
            Self { a, b, c, d }
        }

        #[inline]
        fn evaluate(&self, t: f32) -> f32 {
            let c = 1.0 - t;
            let c2 = c * c;
            let t2 = t * t;
            c2 * c * self.a + 3.0 * c2 * t * self.b + 3.0 * c * t2 * self.c + t2 * t * self.d
        }
    }
}

mod utils {
    use std::ops::Range;

    pub fn rescale(value: f32, old: Range<f32>, new: Range<f32>) -> f32 {
        let Range {
            start: old_min,
            end: old_max,
        } = old;
        let Range {
            start: new_min,
            end: new_max,
        } = new;

        (((value - old_min) * (new_max - new_min)) / (old_max - old_min)) + new_min
    }
}

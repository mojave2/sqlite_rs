mod btree;
mod cursor;
pub mod error;
mod pager;
mod row;
mod table;

use std::num::IntErrorKind;
use std::process;

use lazy_static::lazy_static;
use regex::Regex;

use error::{DbError, ExecErr, MetaCmdErr, PrepareErr};
use row::Row;

pub use table::Table;

pub fn run_cmd(cmd_str: &str, table: &mut Table) -> Result<(), DbError> {
  if cmd_str.starts_with('.') {
    return do_meta_command(cmd_str, table).map_err(DbError::MetaCmdErr);
  }

  let statement = prepare_statement(cmd_str).map_err(DbError::PrepareErr)?;

  execute_statement(&statement, table).map_err(DbError::ExecErr)
}

fn do_meta_command(cmd_str: &str, table: &Table) -> Result<(), MetaCmdErr> {
  match cmd_str {
    ".exit" => {
      table.close_db().unwrap_or_else(|e| {
        eprintln!("{e:?}");
        process::exit(1);
      });
      process::exit(0);
    }
    ".constants" => {
      println!("Constants:");
      print_constants();
    }
    ".btree" => {
      println!("Tree:");
      println!("{}", table.btree_to_str());
    }
    _ => {
      return Err(MetaCmdErr::Unrecognized(format!(
        "Unrecognized command {cmd_str:?}."
      )));
    }
  }
  Ok(())
}

enum Statement {
  Insert(Box<Row>),
  Select,
}

fn prepare_statement(cmd_str: &str) -> Result<Statement, PrepareErr> {
  lazy_static! {
    static ref RE_INSERT: Regex = Regex::new(
      r"(?x)
            insert
            \s+
            (-?\d+)      # id
            \s+
            ([^\s]+)    # username
            \s+
            ([^\s]+)    # email
        "
    )
    .unwrap();
  }
  let syntax_err = "Syntax error. Could not parse statement.".to_string();
  match cmd_str {
    s if s.starts_with("insert") => match RE_INSERT.captures(cmd_str) {
      Some(cap) => {
        let id = match cap[1].parse::<u32>() {
          Ok(v) => v,
          Err(e) if e.kind() == &IntErrorKind::InvalidDigit => {
            return Err(PrepareErr::NegativeId("ID must be positive.".to_string()))
          }
          Err(_) => return Err(PrepareErr::SyntaxErr(syntax_err)),
        };
        Ok(Statement::Insert(Box::new(Row::build(
          id, &cap[2], &cap[3],
        )?)))
      }
      None => Err(PrepareErr::SyntaxErr(syntax_err)),
    },
    s if s.starts_with("select") => Ok(Statement::Select),
    _ => Err(PrepareErr::Unrecognized(format!(
      "Unrecognized keyword at start of {cmd_str:?}."
    ))),
  }
}

fn execute_statement(stmt: &Statement, table: &mut Table) -> Result<(), ExecErr> {
  use Statement::*;
  match stmt {
    Insert(row) => table.insert_row(row.key, row),
    Select => execute_select(table),
  }
}

fn execute_select(table: &mut Table) -> Result<(), ExecErr> {
  let mut cursor = table.new_cursor_by_key(0); // cursor at start of table
  while !cursor.at_end {
    let row = table.select_row(&cursor);
    println!("{row}");
    table.advance_cursor(&mut cursor);
  }
  Ok(())
}

fn print_constants() {
  use btree::leaf::MAX_CELLS;
  println!("ROW_SIZE:                  {}", row::ROW_SIZE);
  println!("LEAF_NODE_MAX_CELLS:       {}", MAX_CELLS);
}

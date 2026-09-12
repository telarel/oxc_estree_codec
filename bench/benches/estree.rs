use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use oxc::allocator::Allocator;
use oxc::span::SourceType;
use sonic_rs::Value;

use oxc_estree_codec::__internal::ProgramReader;

const FIXTURE: &str = r#"import React, { createContext, useContext } from "react";

export interface User {
  readonly id: bigint;
  name: string;
  email?: string | null;
  roles: Array<"admin" | "editor" | "viewer">;
  metadata?: Record<string, unknown>;
}

export type Result<T, E = Error> = { ok: true; value: T } | { ok: false; error: E };

type Handler<E extends Event = Event> = (event: E) => void;

export enum Status {
  Idle = 0,
  Loading = 1,
  Ready = 2,
  Failed = -1,
}

const MAX_RETRIES: number = 5;
const PATTERN: RegExp = /^\/api\/v\d+\/users\/(?<id>\d+)\/?$/gi;
const SCOPE_ID: bigint = 9007199254740993n;

const UserContext = createContext<User | null>(null);

export function useUser(): User {
  const user = useContext(UserContext);
  if (user === null) {
    throw new Error("UserContext is missing a provider");
  }
  return user;
}

export class SessionStore {
  sessions: Map<string, User> = new Map();
  static instance: SessionStore | null = null;
  private listeners: Array<Handler> = [];

  static shared(): SessionStore {
    if (SessionStore.instance === null) {
      SessionStore.instance = new SessionStore();
    }
    return SessionStore.instance;
  }

  get size(): number {
    return this.sessions.size;
  }

  add(key: string, user: User, ...rest: Array<User>): this {
    this.sessions.set(key, user);
    return this;
  }

  async flush(): Promise<Result<number>> {
    try {
      const count = this.sessions.size;
      this.sessions.clear();
      return { ok: true, value: count };
    } catch (error) {
      return { ok: false, error: error as Error };
    } finally {
      for (const listener of this.listeners) {
        listener(new Event("flush"));
      }
    }
  }
}

export function StatusBadge({ status, ...rest }: { status: Status } & Record<string, unknown>) {
  const label =
    status === Status.Failed ? "failed" : status === Status.Loading ? "loading" : "ready";
  return (
    <span className={`badge badge--${label}`} data-status={status} {...rest}>
      {label}
      {status === Status.Failed && <small title="check logs">see trace</small>}
    </span>
  );
}

export const UserCard = ({ user }: { user: User }) => (
  <article className="card">
    <h2>{user.name}</h2>
    <p>{user.email ?? "no email"}</p>
    <ul>
      {user.roles.map((role) => (
        <li key={role}>{role}</li>
      ))}
    </ul>
    <StatusBadge status={Status.Ready} />
    <>{SCOPE_ID}</>
  </article>
);

export async function fetchUsers(
  page = 0,
  ...ids: Array<string>
): Promise<Result<User[]>> {
  const query = `page=${page + 1}&limit=${MAX_RETRIES ?? 10}`;
  const response = await fetch(`/api/v2/users?${query}`);
  const { users, total } = await response.json();
  return total > 0 ? { ok: true, value: users } : { ok: false, error: new Error("empty") };
}
"#;

struct FixtureData {
    json: String,
    value: Value,
    source_type: SourceType,
}

fn prepare() -> FixtureData {
    let source_type: SourceType = SourceType::from_path("fixture.tsx").unwrap();

    let allocator: Allocator = Allocator::default();

    let parser_return: oxc::parser::ParserReturn<'_> =
        oxc::parser::Parser::new(&allocator, FIXTURE, source_type).parse();

    assert!(
        parser_return.diagnostics.is_empty(),
        "bench fixture must parse: {:?}",
        parser_return.diagnostics
    );

    let json: String = parser_return.program.to_estree_json(true, false);

    let value: Value =
        sonic_rs::from_str::<Value>(&json).expect("facade parses the fixture");

    let reader: ProgramReader<'_> = ProgramReader::new(&allocator);

    let program: oxc::ast::ast::Program<'_> = reader
        .read(&value, source_type, FIXTURE)
        .expect("reader round-trips the bench fixture");

    let code: String = oxc::codegen::Codegen::new().build(&program).code;

    assert!(!code.is_empty(), "bench fixture must codegen non-empty output");

    FixtureData { json, value, source_type }
}

fn bench_json_parse(
    criterion: &mut Criterion,
    data: &FixtureData,
) {
    criterion.bench_function("json_parse", |b| {
        b.iter(|| {
            let value: Value =
                sonic_rs::from_str::<Value>(black_box(data.json.as_str()))
                    .unwrap();

            black_box(value)
        })
    });
}

fn bench_ast_read(
    criterion: &mut Criterion,
    data: &FixtureData,
) {
    criterion.bench_function("ast_read", |b| {
        b.iter(|| {
            let allocator: Allocator = Allocator::default();

            let reader: ProgramReader<'_> = ProgramReader::new(&allocator);

            let program: oxc::ast::ast::Program<'_> = reader
                .read(black_box(&data.value), data.source_type, FIXTURE)
                .unwrap();

            let code: String =
                oxc::codegen::Codegen::new().build(&program).code;

            black_box(code.len())
        })
    });
}

fn bench_roundtrip(
    criterion: &mut Criterion,
    data: &FixtureData,
) {
    criterion.bench_function("roundtrip", |b| {
        b.iter(|| {
            let value: Value =
                sonic_rs::from_str::<Value>(black_box(data.json.as_str()))
                    .unwrap();

            let allocator: Allocator = Allocator::default();

            let reader: ProgramReader<'_> = ProgramReader::new(&allocator);

            let program: oxc::ast::ast::Program<'_> =
                reader.read(&value, data.source_type, FIXTURE).unwrap();

            let code: String =
                oxc::codegen::Codegen::new().build(&program).code;

            black_box(code.len())
        })
    });
}

fn bench_estree(criterion: &mut Criterion) {
    let data: FixtureData = prepare();
    bench_json_parse(criterion, &data);
    bench_ast_read(criterion, &data);
    bench_roundtrip(criterion, &data);
}

criterion_group!(benches, bench_estree);
criterion_main!(benches);

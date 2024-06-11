-- Your SQL goes here

CREATE TABLE public.groups (
    uid uuid DEFAULT gen_random_uuid() NOT NULL PRIMARY KEY,
    gid bigint NOT NULL,
    timestart timestamp with time zone NOT NULL,
    interact boolean DEFAULT false NOT NULL,
    mammal numeric,
    fish numeric,
    bird numeric,
    sqlite character varying,
    code character varying,
    comment text,
    timeend timestamp with time zone
);

ALTER TABLE public.groups OWNER TO sealhits;
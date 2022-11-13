/* eslint-disable */
import gql from 'graphql-tag';
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: string;
  String: string;
  Boolean: boolean;
  Int: number;
  Float: number;
  /** Date with time (isoformat) */
  DateTime: any;
  /** The `ID` scalar type represents a unique identifier, often used to refetch an object or as key for a cache. The ID type appears in a JSON response as a String; however, it is not intended to be human-readable. When expected as an input type, any string (such as `"4"`) or integer (such as `4`) input value will be accepted as an ID. */
  GlobalID: any;
};

/** An object with a Globally Unique ID */
export type Node = {
  /** The Globally Unique ID of this object */
  id: Scalars['GlobalID'];
};

export type Organization = Node & {
  __typename?: 'Organization';
  createdAt: Scalars['DateTime'];
  id: Scalars['GlobalID'];
  name: Scalars['String'];
  slug: Scalars['String'];
  updatedAt: Scalars['DateTime'];
};

export type Query = {
  __typename?: 'Query';
  organization?: Maybe<Organization>;
};


export type QueryOrganizationArgs = {
  id: Scalars['GlobalID'];
};

/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: string;
  String: string;
  Boolean: boolean;
  Int: number;
  Float: number;
  /** Date with time (isoformat) */
  DateTime: any;
  /** The `ID` scalar type represents a unique identifier, often used to refetch an object or as key for a cache. The ID type appears in a JSON response as a String; however, it is not intended to be human-readable. When expected as an input type, any string (such as `"4"`) or integer (such as `4`) input value will be accepted as an ID. */
  GlobalID: any;
};

/** An object with a Globally Unique ID */
export type Node = {
  /** The Globally Unique ID of this object */
  id: Scalars['GlobalID'];
};

export type Organization = Node & {
  __typename?: 'Organization';
  createdAt: Scalars['DateTime'];
  id: Scalars['GlobalID'];
  name: Scalars['String'];
  slug: Scalars['String'];
  updatedAt: Scalars['DateTime'];
};

export type Query = {
  __typename?: 'Query';
  organization?: Maybe<Organization>;
};


export type QueryOrganizationArgs = {
  id: Scalars['GlobalID'];
};

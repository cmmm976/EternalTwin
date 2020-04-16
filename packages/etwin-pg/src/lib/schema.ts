export interface UserRow {
  user_id: string;

  ctime: Date;

  display_name: string;

  email_address: string | null;

  email_address_mtime: Date;

  /**
   * Username used to sign-in with a password.
   */
  username: string | null;

  username_mtime: Date;

  password: Uint8Array | null;

  password_mtime: Date;

  is_administrator: boolean;
}

export interface SessionRow {
  session_id: string;

  user_id: string;

  ctime: Date;

  atime: Date;

  data: object;
}